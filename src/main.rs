use anyhow::{anyhow, bail, Context};
use copypasta::{ClipboardContext, ClipboardProvider};
use nitrokey::{Device, GetPasswordSafe, PasswordSafe};
use pinentry::PassphraseInput;
use secrecy::ExposeSecret;
use std::{io::Write, process::Command, process::Stdio, str, thread::sleep, time::Duration};

fn slots(safe: &PasswordSafe) -> anyhow::Result<Vec<Option<String>>> {
    safe.get_slots()?
        .into_iter()
        .map(|slot| slot.map(|slot| slot.get_name()).transpose())
        .collect::<Result<_, _>>()
        .map_err(anyhow::Error::from)
}

fn dmenu(choices: &[Option<String>]) -> anyhow::Result<usize> {
    let mut dmenu = Command::new("dmenu").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    dmenu
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("failed to get stdin of 'dmenu'"))?
        .write_all(choices.iter().flatten().cloned().collect::<Vec<_>>().join("\n").as_bytes())?;
    let output = dmenu.wait_with_output()?;
    if !output.status.success() {
        bail!("'dmenu' exited with {}", output.status);
    }

    let choice = str::from_utf8(&output.stdout)?.trim_end_matches('\n');

    choices
        .iter()
        .enumerate()
        .find_map(|(i, slot)| slot.as_ref().map(|s| s == choice).unwrap_or_default().then_some(i))
        .ok_or_else(|| anyhow!("selected choice not found in store"))
}

fn main() -> anyhow::Result<()> {
    let mut manager = nitrokey::take().context("failed to get device")?;
    let mut device = manager.connect().context("failed to connect to device")?;
    let data = PassphraseInput::with_default_binary()
        .ok_or_else(|| anyhow!("no 'pinentry' command found"))?
        .with_prompt("Nitrokey auth")
        .with_description(&format!(
            "Nitrokey user-pin (tries left {}):",
            device.get_user_retry_count()?
        ))
        .interact()
        .map_err(|e| anyhow!("no password provided: {}", e))?;
    let r = show_dmenu(&mut device, &data);
    device.lock().context("failed to lock device")?;
    r
}

fn show_dmenu(
    device: &mut nitrokey::DeviceWrapper,
    data: &secrecy::SecretString,
) -> anyhow::Result<()> {
    let safe = device.get_password_safe(data.expose_secret())?;

    let choices = slots(&safe)?;
    let choice = dmenu(&choices)?;
    let pw = safe.get_slot(choice as u8)?.get_password()?;
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents(pw).map_err(|e| anyhow!("failed to copy password to clipboard: {}", e))?;
    sleep(Duration::from_secs(5));

    Ok(())
}

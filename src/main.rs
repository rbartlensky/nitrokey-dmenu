use anyhow::{anyhow, bail, Context};
use copypasta::{ClipboardContext, ClipboardProvider};
use nitrokey::{Device, GetPasswordSafe, PasswordSafe};
use pinentry::PassphraseInput;
use secrecy::ExposeSecret;
use std::{
    collections::HashMap, io::Write, process::Command, process::Stdio, thread::sleep,
    time::Duration,
};

fn slots(safe: &PasswordSafe) -> anyhow::Result<HashMap<String, u8>> {
    let mut slots = HashMap::new();
    for (i, slot) in safe.get_slots()?.iter().enumerate() {
        if let Some(slot) = *slot {
            slots.insert(slot.get_name()?, i as u8);
        }
    }
    Ok(slots)
}

fn dmenu(choices: &[String]) -> anyhow::Result<String> {
    let mut dmenu = Command::new("dmenu").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
    dmenu
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow!("failed to get stdin of 'dmenu'"))?
        .write_all(choices.join("\n").as_bytes())?;
    let output = dmenu.wait_with_output()?;
    if !output.status.success() {
        bail!("'dmenu' exited with {}", output.status);
    }
    let choice = String::from_utf8(output.stdout)?;
    Ok(choice.trim_end_matches('\n').into())
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
    let r = slots(&safe);
    if let Ok(slots) = &r {
        let mut choices = slots.iter().map(|(s, _)| s.into()).collect::<Vec<String>>();
        choices.sort();
        let choice = dmenu(&choices)?;
        let pw = safe.get_slot(slots[&choice])?.get_password()?;
        let mut ctx = ClipboardContext::new().unwrap();
        ctx.set_contents(pw).map_err(|e| anyhow!("failed to copy password to clipboard: {}", e))?;
        sleep(Duration::from_secs(5));
    }
    r.map(|_| ())
}

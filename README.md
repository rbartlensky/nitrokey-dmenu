## nitrokey-dmenu

A simple binary that you can use to quickly fetch passwords from your nitrokey.

The app will:
* ask for your nitrokey password
* unlock your nitrokey and fetch all slot names
* feed slot names into dmenu
* get the selected slot and copy your slot's password to the clipboard
* after 5 seconds, the clipboard is cleared, and the nitrokey is locked

## Install

Run `cargo install nitrokey-dmenu`.

### Linking against your system's `libnitrokey`

Before running `cargo install` you can set `USE_SYSTEM_LIBNITROKEY=1`.

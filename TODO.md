### General

- [x] Ensure there's only one instance of the application running
  - ~~A lock file could work, but the problem is, if the app crashed or was force-closed,
    next time, you cannot open the app. because the file couldn't get destroyed on app exit.~~
  - Checking for the TCP address, could be a good way too... if the address/port is in use,
    throw an error.
- [x] Add `confirm_exit` functionality to `Dashboard` app
- [x] Handle IPC between `Service` and `Dashboard`
- [x] Add component values in the `ServerData` (Added `components` to `Application` instead)
- [ ] Add application name to logging system

### Dashboard

- [x] Maybe! a robust system to auto-detect all the components from the device and add them
      to layout. _No promises!_ :D (Hmmm, yeah..., done!)
- [x] Add ability to upload bitmaps from the dashboard to device for "Screen Saver"
  - ~~Only have it when the device and software are paired/connected~~
  - Save it to devive's memory
- [x] Add `State` label for checking if `Dashboard` and `Service` are connected (IPC via TCP)
  - [x] Maybe even one for status of Device-Service connection (Serial)
- [x] Add a modal for `ServerData` fetching errors
- [ ] On every update, send a request to server to update device

### Firmware

- [x] Ground all cd4051be channels to reduce cross-talk and noise
- [x] Map pot values to 0-255 (mapped to 0-99 instead)
- [x] EEPROM/FLASH
  - [x] Make EEPROM-saved keyboard keys/letters usable even if the device wasn't paired
- [x] Pot handling
- [x] Diplay
  - [x] Default screen can be an uploaded bitmap or two strings for `Title` and `Description`
  - [x] Settings menu for device-related stuff. e.g. disable/enable joystick mouse movement
  - [x] Add clock time on the `Home` view and implement a serial function to sync the time
        with software
- [x] Rotary encoder
- [x] Joystick/Thumbstick
  - [x] Add the ability to scroll with the `joystick` if the modkey was `held`

### Optional

- [x] Date/Time formatting for logging system

### PROB-NOT!

- [x] Handle `Display` on a separate core! (Well... that was too easy!)

### FIXME

- [x] Fix multi-core saving bug in which the program crashes if the flash is
      accessed as the display is being updated (FIXED, AGAIN!)
- [x] Fix timing bug with device clock
- [x] Refactor the `ledFlash()` function not to use `delay()` functions
- [x] Saving `home_image` in the flash does not work atm, since we're storing
      the pointer!!!

### README

- [ ] Add "xdg-open" is needed to README.md (open)
- [ ] Add "xdotool" is needed to README.md (enigo)
- [ ] Known Issue: because the ServerData only handles one component at a time,
      if the components are updating too fast for the client to catch up,
      on the `Dashboard` app, the visuals might not update correctly,
      but the functionality of the device and `Service` app remains correct.

### REMINDERS

- ~~There's a hard to reproduce bug in which, if there are profiles with components
  that doesn't have interactions, the config won't load properly, therefore
  the layout resets every time you open the app.
  (It'll probably be fixed when the `new_profile` is implemented)~~

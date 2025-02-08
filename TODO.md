### General

- [x] Ensure there's only one instance of the application running
  - ~~A lock file could work, but the problem is, if the app crashed or was force-closed,
    next time, you cannot open the app. because the file couldn't get destroyed on app exit.~~
  - Checking for the TCP address, could be a good way too... if the address/port is in use,
    throw an error.
- [x] Add `confirm_exit` functionality to `Dashboard` app
- [x] Handle IPC between `Service` and `Dashboard`
- [x] Add component values in the `ServerData` (Added `components` to `Application` instead)
- [x] Add application name to logging system
- [x] Add manual device connection by checking if the `device_name` is empty, forcing app
      to try to connect by the `port_name`
- [ ] Add release builds in the github page
- [x] Cloning the `Config` struct when calling `test_config()` is probably unnecessary. (yep!)
- [x] Add firmware version to the `Firmware` and make it readable by the `Software`

### Dashboard

- [x] Maybe! a robust system to auto-detect all the components from the device and add them
      to layout. _No promises!_ :D (Hmmm, yeah..., done!)
- [x] Add ability to upload bitmaps from the dashboard to device for "Screen Saver"
  - ~~Only have it when the device and software are paired/connected~~
  - Save it to devive's memory
- [x] Add `State` label for checking if `Dashboard` and `Service` are connected (IPC via TCP)
  - [x] Maybe even one for status of Device-Service connection (Serial)
- [x] Add a modal for `ServerData` fetching errors
- [x] On every update, send a request to server to update device
- [x] When updating interaction, it should consider showing correct fields based on current profile
- [x] Create a separate modal for native HID keyboard simulated keys
- [x] Create some nice qol functionalities:
  - [x] Center components to the layout
  - [x] Auto-size layout to fit components
- [x] Add ability to change component id
- [x] Created a parser for the interactions, so when for example the Command contains %value%, it
      gets replaced by the actually value of the component. e.g. potentiometer values
- [ ] Add a welcome screen with some small hints and information about using the app
- [ ] Add `Export/Import` buttons for config file
- [x] Add a small `Information/About` modal that shows info and stuff

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
- [x] Add `home_page` icon for `Potentiometer`

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

###### Service

- [ ] Add "xdg-open" is needed to README.md (open)
- [ ] Add "xdotool" is needed to README.md (enigo)

###### Dashboard

- [ ] Known Issue: because the ServerData only handles one component at a time,
      if the components are updating too fast for the client to catch up,
      on the `Dashboard` app, the visuals might not update correctly,
      but the functionality of the device and `Service` app remains correct.
- [ ] Known Issue: There's a bug (probably from egui side) that a modal with a
      text_edit inside will continiously get larger if the characters in the said
      text_edit exceeds width of the text_edit.
- [ ] Profile '0' is preserved for the device, you are still able to configure it
      the way you want. This is the profile that makes the device act as a keyboard.
      though, you can change each component to do any interaction other than just
      letter press as well.
- [ ] The `Dashboard` app is created for sole purpose of configuring device settings,
      and it is not advised to keep it running as it will consume your resources.

### REMINDERS

- ~~There's a hard to reproduce bug in which, if there are profiles with components
  that doesn't have interactions, the config won't load properly, therefore
  the layout resets every time you open the app.
  (It'll probably be fixed when the `new_profile` is implemented)~~

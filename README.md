# padpad.software

#### PadPad is a customizable macro pad that allows users to execute pre-configured actions through physical components (buttons, knobs, etc.).<br>This repository contains the software that interacts with the PadPad device.

### [🚀 Firmware Repository](https://github.com/IrregularCelery/padpad.firmware) | [📺 YouTube Video (Coming Soon!)]()

## Table of Contents

- [Overview](#-overview)
- [Features](#-key-features)
  - [Dashboard](<#dashboard-(gui-app)>)
  - [Service](<#service-(background-process)>)
- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Using the Dashboard](#-using-the-dashboard)
- [Screenshots](#-screenshots)
- [Known Issues](#-known-issues)
- [Build from Source](#-build-from-source)
- [Contributing](#-contributing)
- [Links](#-links)
- [License](#-license)

## 📖 Overview

**The PadPad software suite consists of two applications:**

1. **Dashboard**: A GUI app to configure device components (buttons, potentiometers, etc.), set interactions, and manage profiles.
2. **Service**: A background app that communicates with the PadPad device to execute interactions.

## ✨ Key Features

### Dashboard (GUI App)

- Configure buttons, potentiometers, interactions, etc.
- Set up multiple profiles for different use cases.
- Assign interactions such as opening applications, running commands, simulating keyboard shortcuts, and more.
- "Button Memory" mode for device-side interactions, even if device isn't paired with the `Service` app.
- Detect and add buttons and potentiometers from the device automatically.

### Service (Background Process)

- Establishes and maintains communication between the device and the software.
- Supports automatic and manual connection modes.
- Can use HID to detect the device by name and determine the correct serial port.
- Ensures seamless operation on Unix-like systems where serial ports might be temporarily inaccessible.

## ⚙️ Installation

### Prebuilt Binaries

Download the latest release for your OS from the [Releases page](https://github.com/IrregularCelery/padpad.software/releases).

> MacOS is untested, but contributions/testing are welcome!

### Linux Dependencies

Ensure the following packages are installed if you're using `X11`:

- Arch Linux (pacman)

  ```bash
  sudo pacman -S xdotool xdg-utils # X11
  ```

- Ubuntu/Debian (apt)

  ```bash
  sudo apt install libxdo-dev xdg-utils # X11
  ```

###### Other distributions will need equivalent packages installed.

> These are required for opening apps/websites and simulating keystrokes. The `xdg-open` utility is used by the [open crate](https://crates.io/crates/open), and `xdotool` is required by the [enigo crate](https://crates.io/crates/enigo).

## 🛠️ Build from Source

1. Clone the repository:

   ```bash
   git clone https://github.com/IrregularCelery/padpad.software.git
   ```

2. Build with Cargo:

   ```bash
   # Service app
   cargo build --release

   # Dashboard app
   cargo build --release --bin dashboard
   ```

   Binaries will be in `target/release/`.

## 🚀 Quick Start

1. **Connect your [PadPad device](https://github.com/IrregularCelery/padpad.firmware)** via usb.
2. **Run the Service app** (keep it running in the background).
3. **Launch the Dashboard app** to configure your device.

## 🎛️ Using the Dashboard

### Creating layout

- Click the large "**`+`**" button to create a layout.
- If you're unsure about the layout size, enter any value. You can use the "`Auto-size Layout`" feature later to adjust it.

### Configuring Components

- Once you've created a layout, two buttons will appear on either side. The button on the right "`+`" opens the "Components" panel, and the button on the left "**`⚙`**" opens the "Toolbar" panel.
- To enter "`Editing Mode`", click the "Components" panel button.
- There are two ways to add components to your layout:
  - Click the **`🗘`** Button to automatically detect the components from your device.<br>**(currently only supports buttons and potentiometers)**
    <br>--- or ---
  - Click any component to manually add it to your layout:
    - **`B`** : **Button**
    - **`L`** : **LED**
    - **`P`** : **Potentiometer**
    - **`J`** : **Joystick**
    - **`R`** : **RotaryEncoder**
    - **`D`** : **Display** (graphical lcd)
      <br>&nbsp;
- In "Editing Mode", click a component in your layout to edit its properties and set up interactions.

  - **Interaction Types**
    - **None**: No software action (useful when configuring "Button Memory").
    - **Command**: Run a shell command.
    - **Application**: Launch an application.
    - **Website**: Open a URL.
    - **Shortcut**: Simulate keypresses (Ctrl+C, etc.) or type text.
    - **File**: Open a specified file.
      <br>&nbsp;

- Make sure to save changes by clicking the **`Save`** button.
  > You can also revert the changes to the last state before entering "Editing Mode".

### Profiles

- Switch profiles from the bottom-right panel.
- Profile `Internal` is reserved for "Button Memory" ('buttons' act as standalone keyboard keys).
  > You can still set up software-based interactions for components in this profile.
- Each profile stores unique component configurations.

### Connection Settings

- Click the connection status indicator (bottom-left) to switch connection to **Manual Mode**, change **Port Name**, **Baud Rate**, etc.

## 🖼️ Screenshots

<details>
  <summary>Show screenshots</summary>

</details>

## ⚠️ Known Issues

## 🤝 Contributing

## 🔗 Links

- [Firmware Setup Guide](https://github.com/IrregularCelery/padpad.firmware)
- [Report a Bug](https://github.com/IrregularCelery/padpad.software/issues)

## 📜 License

**PadPad software** is 100% free and open-source under the [MIT License](/LICENSE).

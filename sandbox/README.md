Sandbox for Temporary Rust Components
=====================================

Welcome to the Sandbox folder for temporary Rust components under development. This folder serves as a safe environment to experiment with and develop new Rust binaries before integrating them into the main project structure.

Purpose
-------

The `sandbox/` folder allows us to:

*   **Test New Components**: Develop and test individual Rust binaries independently.
*   **Organize Experiments**: Organize experimental components and prototypes before integration.
*   **Temporary Development**: Work on features or modules that are not yet finalized or merged into the main project (`src/`).

Template Repository
-------------------

This README file is part of a template repository available on GitHub. You can access the full template and its resources at:

[Link to Template Repository](https://github.com/IrregularCelery/template.rs)

Feel free to explore and adapt the template to suit your Rust project requirements. If you have any questions or feedback, please reach out via GitHub Issues or contact me directly.

Structure
---------

The folder structure is organized as follows:

```
project_root/
├── src/
│   ├── main.rs             <-- main entry point for your main project
│   └── (project files)     <-- rest of the main project
└── sandbox/
    ├── scripts/
    │   └── create.sh       <-- bash script for creating new component
    ├── component1/
    │   └── main.rs         <-- main entry for component 1
    ├── component2/
    │   └── main.rs         <-- main entry for component 2
    └── README.md           <-- this README file
```

### Creating a New Component

To streamline the process of creating and integrating new components into your Rust project, you can use the provided Bash script (`scripts/create.sh`). This script automates the setup of a new component folder with a `main.rs` file and updates the `Cargo.toml` file accordingly.

#### For Unix-like Operating Systems (Linux, macOS)

1.  **Make the Script Executable**: Before running the script for the first time, ensure it has executable permissions:
    
    ```bash
    chmod +x sandbox/scripts/create.sh
    ```
    
2.  **Run the Script**: Execute the script from the root of your project:
    
    ```bash
    ./sandbox/scripts/create.sh
    ```
    
    Follow the prompts to enter the name of the new component. The script will create the necessary folder structure in `sandbox/` and update `Cargo.toml` automatically.
    

#### For Windows Users

If you are using Windows, you can use WSL (Windows Subsystem for Linux) to run the script:

1.  **Install WSL**:
    
    *   If not already installed, follow Microsoft's instructions to install WSL and a Linux distribution of your choice (e.g., Ubuntu).
    *   Open WSL from the Start menu or by typing `wsl` in the command prompt.
2.  **Navigate to Your Project Directory**:
    *   In WSL, navigate to your project directory within the Linux filesystem.
3.  **Execute the Script**:
    *   Follow the instructions for Unix-like systems above, starting from making the script executable and running it.

#### Manual Setup

Alternatively, if you prefer not to use the provided Shell script, you can manually create a new component folder and update `Cargo.toml`:

*   Create a folder (`sandbox/<component_name>/`) manually in your project directory.
*   Create a file named `main.rs` inside the new component folder and add your Rust code.
*   Update `Cargo.toml` with an entry under `[[bin]]` specifying the name and path of the new component (`sandbox/<component_name>/main.rs`).
```
[[bin]]
name = "<component_name>"
path = "sandbox/<component_name>/main.rs"
```

This approach ensures that regardless of your operating system, you can efficiently manage and integrate new components into your Rust project using the provided automation tools or manual steps as needed.

Usage
-----

1.  **Creating Components**: Follow the instructions above to create a new component.
           
2.  **Running Components**: To run the component you can use `cargo run --bin <component_name>` to compile and run the component.
    
    Example:
    
    ```bash
    cargo run --bin component1
    ```
    
3.  **Integration**: Once the component is finalized and ready for integration into the main project (`src/`), move the parts to their appropriate locations and remove them from `sandbox/`.
    

Notes
-----

*   Ensure that dependencies required by sandbox components are managed in `Cargo.toml`.
*   Remove unused or finalized components from version control using Git (`git rm --cached <file>`).

The Sandbox folder encourages interative development and experimentation with your project components, facilitating a structured approach to feature development and integration.

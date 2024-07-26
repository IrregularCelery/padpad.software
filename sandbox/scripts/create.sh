#!/bin/bash

echo "Enter component name:"
read component_name

# Create directory and main.rs file
mkdir -p sandbox/$component_name
touch sandbox/$component_name/main.rs

# Update new component with an example
cat << EOF >> ./sandbox/$component_name/main.rs
fn main() {
    println!("This is a \`sandbox\` component!");
    println!("Component name: \`$component_name\`");
}
EOF

# Update Cargo.toml
cat << EOF >> Cargo.toml

[[bin]]
name = "$component_name"
path = "sandbox/$component_name/main.rs"
EOF

echo "New component '$component_name' created successfully!"


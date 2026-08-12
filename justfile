# Show available recipes
default:
    @just --list

# Format repo
fmt:
    nix fmt

# Big check
check:
    nix flake check

# Clean `target/`
clean:
    cargo clean

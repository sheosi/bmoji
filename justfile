build-flatpak:
    cargo vendor
    flatpak-builder --user --install --force-clean build-dir flatpak/io.github.sheosi.bmoji.yml

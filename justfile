build-flatpak:
    cargo vendor
    flatpak-builder --user --install --force-clean build-dir flatpak/com.sheosi.bmoji.yml

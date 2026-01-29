build-flatpak:
    cargo vendor
    flatpak run --command=flathub-build org.flatpak.Builder --install  io.github.sheosi.bmoji.yml

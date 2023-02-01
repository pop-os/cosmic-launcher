# Cosmic Launcher

Layer Shell frontend for https://github.com/pop-os/launcher. Currently the underlying protocol being used in the plugin for managing toplevels in wayland is defined [here](https://github.com/pop-os/cosmic-protocols/blob/main/unstable/cosmic-toplevel-info-unstable-v1.xml) but it will be switched to use [wlr-foreign-toplevel-management](https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1) when it is ready.

# Building

Cosmic Launcher is set up to build a deb and a Nix flake, but it can be built using meson.

Some Build Dependencies:
```  
  cargo,
  meson,
  intltool,
  appstream-util,
  desktop-file-utils,
  libxkbcommon-dev,
  pkg-config,
  desktop-file-utils,
```


# Translators

Translation files may be found in the i18n directory. New translations may copy the English (en) localization of the project and rename `en` to the desired [ISO 639-1 language code](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes). Translations may be submitted through GitHub as an issue or pull request. Submissions by email or other means are also acceptable; with the preferred name and email to associate with the changes.
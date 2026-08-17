Portable package:

`ui/result/logos-payment_streams_ui-module.lgx`

## Launch Basecamp and load the plugin

Use a Nix-built portable Basecamp (so it matches this `.lgx` and provides `Logos.Theme`). Keep this work off the default user directory.

1. Build portable Basecamp once (skip if you already have `result/bin/LogosBasecamp` from `#bin-bundle-dir`):

```bash
cd /home/sergei/Downloads/software/waku/lez-related/logos-basecamp
nix build '.#bin-bundle-dir'
```

2. Start it with an isolated data dir:

```bash
USER_DIR=/home/sergei/Downloads/software/waku/lez-related/lez-payment-streams/.scaffold/basecamp-ui
mkdir -p "$USER_DIR"
/home/sergei/Downloads/software/waku/lez-related/logos-basecamp/result/bin/LogosBasecamp --user-dir "$USER_DIR"
```

3. In Basecamp, open Package Manager (or Modules), choose Install LGX Package / Install from file, and pick:

`/home/sergei/Downloads/software/waku/lez-related/lez-payment-streams/ui/result/logos-payment_streams_ui-module.lgx`

4. Open Modules, find `payment_streams_ui` under UI modules, click Load. It should appear in the sidebar.

5. Open it, press `test`, confirm the hello popup.

## Rebuild loop

Basecamp does not hot-reload plugins.

```bash
pkill -9 -f 'logos_host|LogosBasecamp'
cd /home/sergei/Downloads/software/waku/lez-related/lez-payment-streams/ui
nix build '.#lgx-portable'
```

Start Basecamp with the same `--user-dir`, uninstall or overwrite `payment_streams_ui`, install the new `.lgx`, Load again.
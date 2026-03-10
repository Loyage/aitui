# TODO

- [ ] NixOS 上按 `y` 键无法复制到剪贴板：已将 reqwest 切换为 rustls-tls 并从 flake.nix 移除 openssl，但仍需在 `nix develop` 环境中测试验证。可能还需要确保 X11/Wayland 相关库在 nix shell 中可用。

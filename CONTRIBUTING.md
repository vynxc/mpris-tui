# Contributing

Issues and focused pull requests are welcome.

Before opening a pull request:

```bash
make check
```

Changes to player discovery or metadata should include unit coverage and, when
appropriate, an isolated mock D-Bus case. UI changes should remain readable in
all four layouts and must not set terminal background colors.


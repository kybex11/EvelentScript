# EvelentScript for VS Code

## Сборка

```shell
build.bat
```

из корня репозитория.

## Иконки (Catppuccin и др.)

Расширение **не меняет** твой File Icon Theme.

С **Catppuccin Icons** `.es` автоматически получает подходящую иконку через `configurationDefaults` (id иконки в теме Catppuccin — legacy-имя для синтаксически совместимых `.es` файлов).

Если стояла сломанная тема `evelent-icons` — верни Catppuccin: **Preferences → File Icon Theme → Catppuccin**.

Другая иконка: Settings → `catppuccin-icons.associations.extensions` → `"es": "javascript"` (или любая из Catppuccin).

Языковые иконки (вкладки): `icons/es-light.svg`, `icons/es-dark.svg`.

## IntelliSense

Output → **EvelentScript**. Установка VSIX 1.3.1+, Reload Window.

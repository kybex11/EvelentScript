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

Output → **EvelentScript**. Установка VSIX 1.6.1+, Reload Window.

### Ложные красные подчёркивания

По умолчанию показываются только ошибки компиляции синтаксиса.  
Семантика TS (undefined name и т.п.) часто ошибается на динамическом `.es` — выключена:

- `evelentscript.diagnostics.semantic`: `false` (по умолчанию)
- Полностью выключить squiggles: `evelentscript.diagnostics.enable`: `false`

### Подсветка не срабатывает

1. В статус-баре язык должен быть **EvelentScript**, не Plain Text / JavaScript.
2. Settings → `files.associations` → `"*.es": "evelentscript"`.
3. Reload Window после установки/обновления расширения.

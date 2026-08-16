# tauri.conf.json

## Что это вообще такое

Главный конфиг Tauri-приложения: пути к фронтенду и dev-серверу,
метаданные приложения, конфигурация окон, бандлинга и security. Один
файл описывает и dev-режим (`tauri dev`), и то, как соберётся финальный
дистрибутив (`tauri build`).

## Когда и как это применяется

Не читается заново при каждом запуске — парсится **на этапе
компиляции** макросом `tauri::generate_context!()` в `lib.rs` и
зашивается в бинарник статической структурой. Поменяли конфиг — нужен
полный рестарт `tauri dev` (не хот-релоад), Rust-процесс должен
пересобраться заново, чтобы подхватить изменения.

Также `tauri-cli` может сам **дописывать** файл в некоторых случаях —
например, если в конфиге стоит `macOSPrivateApi: true`, но в
`Cargo.toml` нет соответствующей cargo-фичи `macos-private-api`,
`tauri-cli` сам её туда добавит при следующем `tauri dev`/`build`.

## Верхнеуровневая структура

```
tauri.conf.json
├── productName / version / identifier   — метаданные приложения
├── build      — dev-сервер, команды сборки фронтенда
├── app        — окна, security, macOS-специфичные глобальные флаги
├── bundle     — иконки, таргеты дистрибутива, платформенные опции
└── plugins    — конфигурация подключённых плагинов (если нужна)
```

Можно завести платформенные оверрайды отдельными файлами —
`tauri.macos.conf.json`, `tauri.windows.conf.json`,
`tauri.linux.conf.json` и т.д. — они мёржатся поверх основного
конфига только на соответствующей платформе.

## `build`

- **`devUrl`** — куда стучится Tauri в dev-режиме
- **`beforeDevCommand`** / **`beforeBuildCommand`** — shell-команда,
  которую `tauri-cli` запускает перед стартом
- **`frontendDist`** — путь к собранным статикам для продакшн-сборки
  (`../dist`), встраивается в бинарник целиком

## `app.windows[]` — конфиг окна

Каждый объект в массиве — одно окно, с обязательным уникальным
`"label"`. `capabilities/*.json` привязываются к окну именно по
этому `label`, не по имени файла (см. `docs/tauri/CAPABILITIES.md`).

**Размер/позиция**: `width`, `height`, `minWidth`/`maxWidth`,
`minHeight`/`maxHeight`, `x`/`y`, `center`, `resizable`.

**Видимость/поведение**: `visible`, `focus` (фокусируется ли при
старте), `focusable` (может ли вообще становиться key window),
`alwaysOnTop`, `alwaysOnBottom`, `skipTaskbar`, `visibleOnAllWorkspaces` и другие.

**Внешний вид**: `decorations` (рамка+тайтлбар в принципе), `transparent`
(требует `macOSPrivateApi: true` на macOS), `shadow`, `titleBarStyle`
(macOS: `Visible`/`Transparent`/`Overlay`), `hiddenTitle`,
`trafficLightPosition` (только с `titleBarStyle: Overlay`).

**Кнопки/хром**: `closable`, `minimizable`, `maximizable` — каждая
управляет наличием соответствующей кнопки в системном стиле окна.

**Mouse/click-поведение (macOS)**: `acceptFirstMouse` — доходит ли
первый клик по неактивному окну до вебвью, или "съедается" системой
как просто активация окна.

**Разное**: `title`, `url` (что грузить — по умолчанию `index.html`),
`userAgent`, `theme`, `dataDirectory`/`dataStoreIdentifier`.

Полный список полей — `WindowConfig` в
[справочнике Tauri](https://v2.tauri.app/reference/config/#windowconfig).

## `app.macOSPrivateApi`

Глобальный флаг (не per-window). Включает приватные API AppKit —
без него `transparent: true` на macOS не работает вообще. Предупреждение
из доки Tauri: использование приватных API не совместимо с публикацией
в App Store.

## `app.security`

- **`csp`** — Content-Security-Policy, инжектится Tauri во все HTML.
  `null` отключает.
- **`capabilities`** — если задать явно, сужает список подключаемых
  capability-файлов (по умолчанию — вообще все `.json` из
  `capabilities/`)

## `bundle`

- **`active`** — собирать ли дистрибутив вообще, или только голый
  исполняемый файл
- **`targets`** — `"all"` или список конкретных
  (`app`/`dmg`/`msi`/`nsis`/`deb`/`rpm`/`appimage`)
- **`icon`** — список путей к иконкам под разные платформы/размеры
  (генерируются командой `tauri icon`, не руками)
- Платформенные под-секции: `macOS` (hardened runtime, entitlements,
  минимальная версия системы), `windows` (сертификат подписи, WiX/NSIS
  настройки), `linux` (deb/rpm/appimage зависимости)

## `plugins`

Объект, где ключ — имя плагина, значение — его собственный конфиг
(структура специфична каждому плагину).

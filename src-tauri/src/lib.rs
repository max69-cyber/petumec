use tauri::Manager;

/// ⚠️⚠️ DANGER ZONE - НЕ ЛЕЗТЬ БЕЗ КРАЙНЕЙ НЕОБХОДИМОСТИ ⚠️⚠️
///
/// Ретегирует нативный NSWindow в настоящий NSPanel с семантикой
/// NonactivatingPanel — по рецепту из native-sdk (macOS AppKit host,
/// see NativeSdkCompanionPanel в appkit_host.m): обычный NSWindow
/// формально может попасть в fullscreen-Space, но WindowServer рисует
/// его позади контента fullscreen-приложения, а не оборачивает окно —
/// и только реальный NSPanel с NonactivatingPanel не ворует активацию
/// приложения при клике. Оба свойства зависят от настоящего класса
/// объекта, а не только от битов styleMask — отсюда подмена класса
/// через object_setClass.
///
/// Выполняется с задержкой на один тик run loop через dispatch_async на
/// главную очередь — тот же паттерн, что использовался в самом первом
/// SwiftUI-прототипе (`DispatchQueue.main.async { configure(view) }`).
/// Краш-репорты показали, что синхронная версия падала внутри
/// собственного `did_finish_launching` у `tao`, который донастраивает
/// окно уже *после* того, как `setup()` вернул управление, но всё ещё
/// в том же коллбэке — деференс даёт этому коду полностью отработать
/// прежде, чем мы вообще трогаем окно.
///
/// ⚠️⚠️ DANGER ZONE - НЕ ЛЕЗТЬ БЕЗ КРАЙНЕЙ НЕОБХОДИМОСТИ ⚠️⚠️
#[cfg(target_os = "macos")]
fn make_pet_overlay(window: &tauri::WebviewWindow) {
    // функция подмены класса объекта в рантайме
    use objc2::ffi::object_setClass;
    // тип любого объекта objc2, его принимает метод object_setClass
    use objc2::runtime::AnyObject;
    // Трейт (типа интерфейс), который позволяет вызвать у NSPanel метод, отдающий его класс - NSPanel::class()
    use objc2::ClassType;
    // NSPanel — сам тип окна. NSWindowCollectionBehavior/NSWindowStyleMask -
    // битовые флаги, которыми настраивается поведение окна.
    use objc2_app_kit::{NSPanel, NSWindowCollectionBehavior, NSWindowStyleMask};
    // Указатель на что угодно - не важно какой тип на том конце
    use std::ffi::c_void;

    // Символы GCD (libdispatch) — сама очередь и функция постановки в неё.
    extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }

    // Реальная настройка окна — выполнится не сразу, а через dispatch_async_f ниже.
    extern "C" fn configure(context: *mut c_void) {
        unsafe {
            let ns_window_ptr = context as *mut AnyObject;
            object_setClass(ns_window_ptr, NSPanel::class() as *const _);

            let panel: &NSPanel = &*(ns_window_ptr as *const NSPanel);

            // Titled + FullSizeContentView уже выставлены через tauri.conf.json
            // (decorations:true + titleBarStyle:"Overlay") — просто добавляем
            // бит NonactivatingPanel, не перезаписывая маску целиком.
            let style_mask = panel.styleMask();
            panel.setStyleMask(style_mask | NSWindowStyleMask::NonactivatingPanel);
            // titlebarAppearsTransparent / titleVisibility убраны — уже
            // покрыты titleBarStyle:"Overlay" / hiddenTitle:true в конфиге.
            // setOpaque(false) / setHasShadow(false) убраны — уже покрыты
            // transparent:true / shadow:false в tauri.conf.json.
            panel.setFloatingPanel(true);
            panel.setBecomesKeyOnlyIfNeeded(true);
            panel.setHidesOnDeactivate(false);

            // CanJoinAllSpaces убран — уже покрыт visibleOnAllWorkspaces:true
            // в tauri.conf.json.
            let behavior = panel.collectionBehavior();
            panel.setCollectionBehavior(
                behavior
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::Stationary
                    | NSWindowCollectionBehavior::CanJoinAllApplications,
            );

            // NSScreenSaverWindowLevel — проверено эмпирически: одного
            // alwaysOnTop (NSFloatingWindowLevel из конфига) недостаточно
            // для видимости над fullscreen-Space — убирали эту строку,
            // видимость пропадала.
            panel.setLevel(1000);
        }
    }

    // Достаём сырой указатель на нативное окно и откладываем всю
    // настройку на следующий тик run loop.
    let _ = window.with_webview(|webview| unsafe {
        let ns_window_ptr = webview.ns_window() as *mut c_void;
        dispatch_async_f(&_dispatch_main_q as *const c_void, ns_window_ptr, configure);
    });
}

// отвечает за иконку в менюбаре + выпадающее из нее меню
// Result пустой, потому что в крейте tauri зашили внутрь Err уже значение
fn make_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::image::Image;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    // создаем айтемы для меню:
    // аргументы: apphandle, айди айтема, лейбл айтема, флаг включенности, хоткей для айтема
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    // и создаем само меню
    let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

    // зашиваем PNG в бинарник на этапе компиляции (include_bytes!) — не
    // зависим от путей на диске у собранного .app, работает одинаково
    // и в tauri dev, и в финальной сборке.
    // TODO: изучить подробнее темку
    let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
        .expect("tray-icon.png должен быть валидным PNG");

    // создает иконку из темплейта:
    TrayIconBuilder::new()
        // сначала добавим саму картинку - .icon
        // потом включаем особую функцию для macos - перезаливка белым/черным в зависимости от фона - .icon_as_template
        // в меню нужна ссылка на menu - .menu
        // дальше пишем обработчики, .on_menu_event
        // .build завершает цепочку и регает в систему эту менюшку.
        .icon(tray_icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            // берем хэндл app и event, в котором есть id айтема, вот его и достаем как &str (.as_ref), чтобы работать
            // нормально со строковыми литералами. потом каждый id расписываем ->
            "settings" => {
                println!("settings clicked");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app
                .get_webview_window("pet-overlay")
                .ok_or("window with pet-overlay identifier is not found")?;

            #[cfg(target_os = "macos")]
            make_pet_overlay(&window);

            // handle тут - это набор методов оригинального app, только те которые более долгоживущие,
            // не одноразовые, по типу run, а универсальные, например resize для окна
            make_tray(app.handle())?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

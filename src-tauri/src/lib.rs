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

            // ПЕРВЫЙ показ окна — не ре-ордер. В tauri.conf.json у
            // pet-overlay стоит visible:false, поэтому tao при создании окна
            // пропускает свой orderFront (window.rs:630 — показ обёрнут в
            // `if visible`), а CanJoinAllSpaces из visibleOnAllWorkspaces всё
            // равно применяет. Так убрана гонка: WindowServer впервые видит
            // окно уже полноценным NSPanel со всеми флагами и уровнем, а не
            // обычным NSWindow, каким оно было до этого коллбэка.
            //
            // Гонка проявлялась так: запускаешь приложение, когда активен
            // чужой fullscreen-Space — спрайта не видно; уходишь на обычный
            // Space — видно; и только после первого перетаскивания (драг =
            // move + reorder, то есть принудительный пересчёт членства в
            // Spaces) питомец начинал показываться над fullscreen. Смена
            // level/collectionBehavior задним числом это членство не
            // пересчитывает, поэтому компенсировать ре-ордером не вышло —
            // показывать надо сразу с правильными флагами.
            //
            // orderFrontRegardless, а не tauri-шный window.show(): show()
            // разворачивается в make_key_and_order_front_sync (window.rs:670)
            // и сделает окно key, то есть украдёт фокус. Голый orderFront
            // тоже не годится — он срабатывает только когда приложение уже
            // активно, а питомец активации не получает.
            panel.orderFrontRegardless();
        }
    }

    // Достаём сырой указатель на нативное окно и откладываем всю
    // настройку на следующий тик run loop.
    let result = window.with_webview(|webview| unsafe {
        let ns_window_ptr = webview.ns_window() as *mut c_void;
        dispatch_async_f(&_dispatch_main_q as *const c_void, ns_window_ptr, configure);
    });

    // Фолбэк на случай, когда до нативного окна вообще не добрались.
    // Окно создаётся с visible:false и показывается только в конце
    // configure — значит если with_webview упал, configure не поставится
    // в очередь и приложение молча запустится без видимого питомца.
    // Лучше показать окно с неправильным поведением, чем не показать
    // ничего: питомца будет видно и приложение можно будет закрыть.
    //
    // Тут именно show(), хотя он и делает окно key (крадёт фокус) — на
    // этом пути настроить окно уже не получится, так что выбираем
    // видимость, а не отсутствие фокуса.
    if let Err(err) = result {
        eprintln!("failed to reach the native pet window, showing it unconfigured: {err}");
        let _ = window.show();
    }
}

/// Настраивает окно настроек так, чтобы оно вело себя правильно
/// относительно Spaces и fullscreen. В отличие от питомца тут НЕ нужен
/// ни NSPanel, ни поднятый level: окно должно остаться обычным key-окном
/// с нормальной активацией, иначе сломается ввод в полях.
///
/// Само появление настроек поверх чужого fullscreen обеспечивает не это,
/// а ActivationPolicy::Accessory на уровне всего приложения (см. run()):
/// у accessory-приложения нет своего Space, поэтому его обычное окно
/// рождается в том Space, который активен прямо сейчас.
///
/// Здесь закрывается только один остаточный случай: окно настроек уже
/// открыто, пользователь ушёл в fullscreen и снова жмёт "Settings".
/// NSWindow живёт в том Space, где был создан, поэтому без MoveToActiveSpace
/// macOS перекинул бы пользователя обратно из fullscreen на старый десктоп.
///
/// MoveToActiveSpace несовместим с CanJoinAllSpaces — для окна настроек
/// visibleOnAllWorkspaces включать нельзя.
///
/// Синхронно, без dispatch_async: окно создаётся по клику в трее, то есть
/// сильно позже did_finish_launching у tao — гонки, из-за которой пришлось
/// откладывать настройку питомца, тут нет.
#[cfg(target_os = "macos")]
fn configure_settings_window(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

    let Ok(ns_window_ptr) = window.ns_window() else {
        return;
    };

    unsafe {
        let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);
        let behavior = ns_window.collectionBehavior();
        ns_window.setCollectionBehavior(
            behavior
                | NSWindowCollectionBehavior::MoveToActiveSpace
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

// Открывает окно настроек по клику на пункт меню "Settings".
// Если окно уже создано (пользователь открывал его раньше в этой
// сессии) — просто фокусируем существующее, не плодим дубли.
// Закрытие крестиком в tauri по умолчанию уничтожает окно, так что
// в эту ветку попадаем только когда настройки реально открыты.
// TODO: сейчас грузит index.html (страница питомца) как временную
// заглушку — переключить на отдельную точку входа (settings.html),
// когда появится фронтенд настроек.
fn open_settings_window(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        // set_focus разворачивается в makeKeyAndOrderFront + [NSApp activate];
        // вместе с MoveToActiveSpace это подтягивает окно в текущий Space.
        let _ = window.set_focus();
        return;
    }

    // Обычное окно: декорации, только кнопка закрытия (без свернуть/
    // развернуть), не resizable. Никакого NSPanel и alwaysOnTop — видимость
    // поверх fullscreen даёт Accessory-политика приложения.
    let result = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("petumec — settings")
        .inner_size(400.0, 300.0)
        .resizable(false)
        .decorations(true)
        .closable(true)
        .minimizable(false)
        .maximizable(false)
        .build();

    // on_menu_event не даёт пробросить ошибку через ? (замыкание не
    // возвращает Result) — если создание окна упало, просто пишем в
    // лог, не роняя всё приложение.
    match result {
        Ok(_window) => {
            #[cfg(target_os = "macos")]
            configure_settings_window(&_window);
        }
        Err(err) => eprintln!("failed to open settings window: {err}"),
    }
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
        .expect("tray-icon.png must be a valid PNG");

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
            "settings" => open_settings_window(app),
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

            // Политика активации всего приложения (не per-window). Regular-
            // приложение macOS исключает из fullscreen-Space чужого приложения,
            // поэтому companion-апп обязан быть Accessory — без этого окно
            // настроек не появится поверх fullscreen, а выкинет пользователя
            // из него на наш десктоп.
            //
            // Цена: нет иконки в Dock и нет строки меню. Вход в приложение
            // остаётся через трей. Отсутствие строки меню значит, что
            // Cmd+C/V/A/Q в окне настроек могут не работать — если понадобятся,
            // придётся ставить NSApp.mainMenu руками.
            #[cfg(target_os = "macos")]
            let _ = app
                .handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory);

            // handle тут - это набор методов оригинального app, только те которые более долгоживущие,
            // не одноразовые, по типу run, а универсальные, например resize для окна
            make_tray(app.handle())?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

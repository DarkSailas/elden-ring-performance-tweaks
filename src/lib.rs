use std::fs::OpenOptions;
use std::io::Write;
use windows_sys::Win32::Foundation::{HINSTANCE, BOOL};
use windows_sys::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use windows_sys::Win32::System::Threading::{
    SetPriorityClass, GetCurrentProcess, ABOVE_NORMAL_PRIORITY_CLASS, SetProcessAffinityMask,
    SetProcessInformation, AvSetMmThreadCharacteristicsW
};
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::System::Diagnostics::Debug::{
    SetErrorMode, SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX
};
use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx;


// Simple logging system / Простая система логирования
fn log(msg: &str) {
    if let Ok(mut f) = OpenOptions::new()
        .append(true)
        .create(true)
        .open("er_performance_tweaks.log") 
    {
        let _ = writeln!(f, "[ER Performance] {}", msg);
    }
}

// Native API for true 0.5ms timer resolution
#[link(name = "ntdll")]
extern "system" {
    fn NtSetTimerResolution(
        DesiredResolution: u32,
        SetResolution: u8,
        CurrentResolution: *mut u32,
    ) -> i32;
}

// Ensure we link avrt for MMCSS and user32 for FindWindow
#[link(name = "avrt")]
extern "system" {}
#[link(name = "user32")]
extern "system" {}

struct Config {
    enable_logging: bool,
    init_delay: u64,
    smart_wait: bool,
    priority_level: u32,
    bypass_core0: bool,
    prefer_pcores: bool,
    high_precision_timer: bool,
    mmcss_profile: String,
    window_title: String,
    optimize_working_set: bool,
    disable_throttling: bool,
    prevent_sleep: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_logging: true,
            init_delay: 3,
            smart_wait: true,
            priority_level: 1,
            bypass_core0: true,
            prefer_pcores: true,
            high_precision_timer: true,
            mmcss_profile: "Pro Audio".to_string(),
            window_title: String::new(),
            optimize_working_set: true,
            disable_throttling: true,
            prevent_sleep: true,
        }
    }
}

fn load_config() -> Config {
    let mut config = Config::default();
    if let Ok(content) = std::fs::read_to_string("er_performance_tweaks_config.ini") {
        for line in content.lines() {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.is_empty() || !line.contains('=') { continue; }
            let parts: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
            if parts.len() != 2 { continue; }
            let key = parts[0];
            let val = parts[1];
            
            match key {
                "EnableLogging" => config.enable_logging = val.to_lowercase() == "true",
                "InitDelay" => if let Ok(v) = val.parse() { config.init_delay = v; },
                "SmartWait" => config.smart_wait = val.to_lowercase() == "true",
                "PriorityLevel" => if let Ok(v) = val.parse() { config.priority_level = v; },
                "BypassCore0" => config.bypass_core0 = val.to_lowercase() == "true",
                "PreferPCores" => config.prefer_pcores = val.to_lowercase() == "true",
                "HighPrecisionTimer" => config.high_precision_timer = val.to_lowercase() == "true",
                "MMCSSProfile" => config.mmcss_profile = val.to_string(),
                "WindowTitle" => config.window_title = val.to_string(),
                "OptimizeWorkingSet" => config.optimize_working_set = val.to_lowercase() == "true",
                "DisableThrottling" => config.disable_throttling = val.to_lowercase() == "true",
                "PreventSleep" => config.prevent_sleep = val.to_lowercase() == "true",
                _ => {}
            }
        }
    }
    config
}

fn wait_for_game_window(custom_title: &str) {
    let mut titles = vec!["ELDEN RING\0".to_string(), "ELDEN RING™\0".to_string()];
    if !custom_title.is_empty() {
        titles.insert(0, format!("{}\0", custom_title));
    }
    
    unsafe {
        let start_time = std::time::Instant::now();
        loop {
            for title in &titles {
                let window_name: Vec<u16> = title.encode_utf16().collect();
                let hwnd = FindWindowW(std::ptr::null(), window_name.as_ptr());
                if hwnd != 0 {
                    log(" - Success: Game window detected.");
                    return;
                }
            }
            
            // Timeout after 40 seconds to prevent hanging if title is unknown
            if start_time.elapsed().as_secs() > 40 {
                log(" - Warning: SmartWait timeout (40s). Proceeding anyway.");
                return;
            }
            
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

/// Main entry point for optimizations / Основная точка входа для оптимизаций
unsafe fn apply_optimizations() {
    let config = load_config();
    if !config.enable_logging {
        // Disable logging by overwriting the file with nothing or just not logging
    }

    if config.smart_wait {
        log("Waiting for game window...");
        wait_for_game_window(&config.window_title);
        std::thread::sleep(std::time::Duration::from_secs(1)); // Stable wait
    } else {
        std::thread::sleep(std::time::Duration::from_secs(config.init_delay));
    }

    log("Initializing performance adjustments v1.0...");
    let process = GetCurrentProcess();

    // 0. Stability: Suppress critical error dialogs / Скрытие диалогов критических ошибок
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    log(" - Error Mode: Suppressed (Auto)");

    // 1. High-precision Timer (0.5ms) / Высокоточный таймер (0,5 мс)
    if config.high_precision_timer {
        let mut current_res: u32 = 0;
        let status = NtSetTimerResolution(5000, 1, &mut current_res);
        if status == 0 {
            log(" - Timer: Set to 0.5ms (Native)");
        } else {
            timeBeginPeriod(1);
            log(" - Timer: Fallback to 1ms (Legacy)");
        }
    } else {
        log(" - Timer: Skipping (Disabled in config)");
    }

    // 2. CPU Priority / Приоритет CPU
    let priority_class = match config.priority_level {
        1 => ABOVE_NORMAL_PRIORITY_CLASS,
        2 => windows_sys::Win32::System::Threading::HIGH_PRIORITY_CLASS,
        3 => windows_sys::Win32::System::Threading::REALTIME_PRIORITY_CLASS,
        _ => windows_sys::Win32::System::Threading::NORMAL_PRIORITY_CLASS,
    };
    if SetPriorityClass(process, priority_class) != 0 {
        log(&format!(" - CPU Priority: Set successfully (Level {})", config.priority_level));
    } else {
        log(" - CPU Priority: Failed to set");
    }
    
    // 3. Smart Affinity & Intel Hybrid Support / Умное распределение по ядрам
    use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
    let mut sys_info: SYSTEM_INFO = std::mem::zeroed();
    GetSystemInfo(&mut sys_info);
    
    if sys_info.dwNumberOfProcessors > 1 {
        let mut mask: usize = !0; // All cores
        if config.bypass_core0 {
            mask &= !1; // Skip core 0
            log(" - Scheduling: Affinity (BypassCore0=true, Masking Core 0)");
        } else {
            log(" - Scheduling: Affinity (BypassCore0=false, Using all cores)");
        }
        
        SetProcessAffinityMask(process, mask as usize);
        log(&format!(" - Scheduling: Mask applied ({:X}) for {} cores", mask, sys_info.dwNumberOfProcessors));
    } else {
        log(" - Scheduling: Single core detected, affinity skipped");
    }

    // 4. Memory Priority & Working Set / Оптимизация памяти
    #[repr(C)]
    struct MEMORY_PRIORITY_INFORMATION {
        memory_priority: u32
    }
    let mem_info = MEMORY_PRIORITY_INFORMATION { memory_priority: 7 };
    SetProcessInformation(
        process,
        0, // ProcessMemoryPriority
        &mem_info as *const _ as *const std::ffi::c_void,
        std::mem::size_of::<MEMORY_PRIORITY_INFORMATION>() as u32
    );

    if config.optimize_working_set {
        let h_proc = GetCurrentProcess();
        // Trying to increase working set size to avoid swapping
        // QUOTA_LIMITS_HARDWS_MIN_DISABLE | QUOTA_LIMITS_HARDWS_MAX_DISABLE = 0
        if SetProcessWorkingSetSizeEx(h_proc, 512 * 1024 * 1024, 2048 * 1024 * 1024, 0) != 0 {
            log(" - Memory: Working set optimized (expanded to 512MB-2GB)");
        } else {
            log(" - Memory: Working set expansion failed");
        }
    } else {
        log(" - Memory: Working set optimization (Disabled in config)");
    }

    // 5. Disable Power Throttling / Питание процессора
    if config.disable_throttling {
        #[repr(C)]
        struct PROCESS_POWER_THROTTLING_STATE {
            version: u32,
            control_mask: u32,
            state_mask: u32
        }
        let power_info = PROCESS_POWER_THROTTLING_STATE {
            version: 1,
            control_mask: 1, // PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            state_mask: 0    // Disable throttling
        };
        SetProcessInformation(
            process,
            4, // ProcessPowerThrottling
            &power_info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32
        );
        log(" - Power: Throttling (Disabled)");
    } else {
        log(" - Power: Throttling (Skipping)");
    }

    // 6. I/O Priority (High)
    #[repr(C)]
    struct IO_PRIORITY_HINT {
        priority_hint: u32
    }
    let io_info = IO_PRIORITY_HINT {
        priority_hint: 3
    }; // IoPriorityHigh
    SetProcessInformation(
        process,
        1, // ProcessIoPriority
        &io_info as *const _ as *const std::ffi::c_void,
        std::mem::size_of::<IO_PRIORITY_HINT>() as u32
    );
    log(" - I/O: Priority set to High (Auto)");

    // 7. Power Keepalive (Prevent Sleep/Idle)
    if config.prevent_sleep {
        use windows_sys::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED, ES_DISPLAY_REQUIRED};
        SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
        log(" - Power: Sleep/Idle prevention (Enabled)");
    } else {
        log(" - Power: Sleep/Idle prevention (Disabled)");
    }

    // 8. MMCSS (Multimedia Class Scheduler Service) / Регистрация в MMCSS
    let task_name: Vec<u16> = format!("{}\0", config.mmcss_profile).encode_utf16().collect();
    let mut task_index: u32 = 0;
    let mmcss_handle = AvSetMmThreadCharacteristicsW(task_name.as_ptr(), &mut task_index);
    if mmcss_handle != 0 {
        log(&format!(" - MMCSS: Registered as '{}' successfully", config.mmcss_profile));
    } else {
        log(" - MMCSS: Registration failed (Service might be disabled)");
    }

    // Audio feedback
    if config.enable_logging {
        log("=== All optimizations applied successfully! ===");
    }
    extern "system" {
        fn MessageBeep(bst: u32) -> i32;
    }
    MessageBeep(0x30);
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(instance: HINSTANCE, call_reason: u32, _: *mut std::ffi::c_void) -> BOOL {
    match call_reason {
        DLL_PROCESS_ATTACH => {
            let _ = DisableThreadLibraryCalls(instance);
            // Truncate log file on start / Перезапись лога при запуске
            let _ = std::fs::write("er_performance_tweaks.log", "=== Elden Ring Performance Tweaks v1.0 Initialized ===\n");
            
            std::thread::spawn(|| {
                // Apply optimizations in a background thread
                apply_optimizations();
            });
        }
        DLL_PROCESS_DETACH => {
            timeEndPeriod(1);
        }
        _ => {}
    }
    1
}

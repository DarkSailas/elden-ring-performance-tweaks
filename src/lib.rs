use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{HINSTANCE, BOOL};
use windows_sys::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameW};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use windows_sys::Win32::System::Threading::{
    SetPriorityClass, GetCurrentProcess, ABOVE_NORMAL_PRIORITY_CLASS, SetProcessAffinityMask,
    SetProcessInformation, AvSetMmThreadCharacteristicsW
};
use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows_sys::Win32::System::Diagnostics::Debug::{
    SetErrorMode, SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX
};
use windows_sys::Win32::System::Memory::{
    SetProcessWorkingSetSizeEx,
    QUOTA_LIMITS_HARDWS_MIN_DISABLE, QUOTA_LIMITS_HARDWS_MAX_DISABLE
};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use std::sync::OnceLock;
use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

static mut G_DLL_INSTANCE: HINSTANCE = 0;
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

fn get_dll_dir() -> PathBuf {
    unsafe {
        let mut buffer = vec![0u16; 1024];
        let len = GetModuleFileNameW(G_DLL_INSTANCE, buffer.as_mut_ptr(), buffer.len() as u32);
        if len > 0 {
            let s = String::from_utf16_lossy(&buffer[..len as usize]);
            let mut path = PathBuf::from(s);
            path.pop();
            return path;
        }
    }
    PathBuf::new()
}

fn get_exe_dir() -> PathBuf {
    unsafe {
        let mut buffer = vec![0u16; 1024];
        let len = GetModuleFileNameW(0, buffer.as_mut_ptr(), buffer.len() as u32);
        if len > 0 {
            let s = String::from_utf16_lossy(&buffer[..len as usize]);
            let mut path = PathBuf::from(s);
            path.pop();
            return path;
        }
    }
    PathBuf::new()
}

fn find_config_path() -> PathBuf {
    CONFIG_PATH.get_or_init(|| {
        // 1. Try DLL directory first (mod\dll\er_performance_tweaks_config.ini)
        let mut p = get_dll_dir();
        p.push("er_performance_tweaks_config.ini");
        if p.exists() {
            return p;
        }

        // 2. Try Game exe directory as fallback
        let mut p2 = get_exe_dir();
        p2.push("er_performance_tweaks_config.ini");
        if p2.exists() {
            return p2;
        }

        p
    }).clone()
}

fn get_log_path() -> &'static PathBuf {
    LOG_PATH.get_or_init(|| {
        let mut path = get_dll_dir();
        if !path.exists() {
            path = get_exe_dir();
        }
        path.push("er_performance_tweaks_log.log");
        path
    })
}

// Simple logging system / Простая система логирования
fn log(msg: &str) {
    let path = get_log_path();
    if let Ok(mut f) = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path) 
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

pub struct Config {
    pub enable_logging: bool,
    pub init_delay: u64,
    pub smart_wait: bool,
    pub priority_level: u32,
    pub bypass_core0: bool,
    pub prefer_pcores: bool,
    pub high_precision_timer: bool,
    pub mmcss_profile: String,
    pub window_title: String,
    pub optimize_working_set: bool,
    pub disable_throttling: bool,
    pub prevent_sleep: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_logging: false,
            init_delay: 3,
            smart_wait: true,
            priority_level: 1,
            bypass_core0: false, // Default to FALSE: preserve Core 0 for game threads
            prefer_pcores: true,
            high_precision_timer: true,
            mmcss_profile: "Games".to_string(),
            window_title: String::new(),
            optimize_working_set: false, // Default to FALSE to prevent 2GB hard swapping
            disable_throttling: true,
            prevent_sleep: true,
        }
    }
}

pub fn load_config() -> Config {
    let mut config = Config::default();
    let config_path = find_config_path();
    
    if let Ok(content) = std::fs::read_to_string(&config_path) {
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

    if config.smart_wait {
        log("Waiting for game window...");
        wait_for_game_window(&config.window_title);
        std::thread::sleep(std::time::Duration::from_secs(1)); // Stable wait
    } else {
        std::thread::sleep(std::time::Duration::from_secs(config.init_delay));
    }

    log("Initializing performance adjustments v1.1...");
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
    
    // 3. Smart Affinity & Thread Scheduling
    // When bypass_core0 is false, we DO NOT call SetProcessAffinityMask, preserving OS Thread Director & P/E-core mapping!
    if config.bypass_core0 {
        use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
        let mut sys_info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut sys_info);
        
        if sys_info.dwNumberOfProcessors > 1 {
            let mask: usize = (!0usize) & (!1usize); // Skip Core 0
            SetProcessAffinityMask(process, mask);
            log(&format!(" - Scheduling: Affinity (BypassCore0=true, Mask applied: {:X})", mask));
        }
    } else {
        log(" - Scheduling: OS Managed (All CPU cores & Thread Director preserved)");
    }

    // 4. Memory Priority & Dynamic Working Set Expansion (Zero Hard-Caps!)
    #[repr(C)]
    struct MEMORY_PRIORITY_INFORMATION {
        memory_priority: u32,
    }
    let mem_info = MEMORY_PRIORITY_INFORMATION { memory_priority: 7 };
    SetProcessInformation(
        process,
        0, // ProcessMemoryPriority (Highest)
        &mem_info as *const _ as *const std::ffi::c_void,
        std::mem::size_of::<MEMORY_PRIORITY_INFORMATION>() as u32,
    );

    if config.optimize_working_set {
        let h_proc = GetCurrentProcess();
        let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(&mut mem_status) != 0 {
            let total_ram = mem_status.ullTotalPhys;
            // Dynamically scale minimum working set: 4GB on 16GB+ systems, 2GB on 8GB systems
            let min_ws: usize = if total_ram >= 16 * 1024 * 1024 * 1024 {
                4 * 1024 * 1024 * 1024
            } else if total_ram >= 8 * 1024 * 1024 * 1024 {
                2 * 1024 * 1024 * 1024
            } else {
                1024 * 1024 * 1024
            };

            // CRITICAL: QUOTA_LIMITS_HARDWS_MIN_DISABLE | QUOTA_LIMITS_HARDWS_MAX_DISABLE = 12
            // Setting Flags = 12 disables hard page quota limits, allowing Elden Ring to consume 6-10GB without disk swapping!
            let flags = QUOTA_LIMITS_HARDWS_MIN_DISABLE | QUOTA_LIMITS_HARDWS_MAX_DISABLE;
            if SetProcessWorkingSetSizeEx(h_proc, min_ws, usize::MAX, flags) != 0 {
                log(&format!(" - Memory: Dynamic working set expanded (min {} MB, soft max unlimited)", min_ws / (1024 * 1024)));
            } else {
                log(" - Memory: Dynamic working set expansion failed");
            }
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
            state_mask: u32,
        }
        let power_info = PROCESS_POWER_THROTTLING_STATE {
            version: 1,
            control_mask: 1, // PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            state_mask: 0,   // Disable throttling
        };
        SetProcessInformation(
            process,
            4, // ProcessPowerThrottling
            &power_info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
        log(" - Power: Throttling (Disabled)");
    } else {
        log(" - Power: Throttling (Skipping)");
    }

    // 6. I/O Priority (High)
    #[repr(C)]
    struct IO_PRIORITY_HINT {
        priority_hint: u32,
    }
    let io_info = IO_PRIORITY_HINT {
        priority_hint: 3, // IoPriorityHigh
    };
    SetProcessInformation(
        process,
        1, // ProcessIoPriority
        &io_info as *const _ as *const std::ffi::c_void,
        std::mem::size_of::<IO_PRIORITY_HINT>() as u32,
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

    // 8. MMCSS (Multimedia Class Scheduler Service)
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
            G_DLL_INSTANCE = instance;
            let _ = DisableThreadLibraryCalls(instance);
            
            // Truncate log file on start
            let log_file = get_log_path();
            let _ = std::fs::write(log_file, "=== Elden Ring Performance Tweaks v1.1.0 Initialized ===\n");
            
            let _ = std::thread::Builder::new()
                .name("er-perf-tweaks".to_string())
                .stack_size(2 * 1024 * 1024)
                .spawn(|| {
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

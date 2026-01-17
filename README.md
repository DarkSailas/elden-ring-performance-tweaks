# 🐉 Elden Ring Performance Tweaks

![Version](https://img.shields.io/badge/version-1.0.0-gold?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Windows-blue?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-green?style=for-the-badge)

[Русский](#russian) | [English](#english)

---

<a name="english"></a>

## ⚔️ English

A lightweight, high-performance enhancement DLL designed to eliminate stutters and optimize engine behavior in **Elden Ring**.

> [!IMPORTANT]
> **This tool is for WINDOWS ONLY.** It relies on low-level Windows APIs for memory and CPU scheduling.

### ✨ Key Features
- ⚙️ **External Configuration**: Full control via `er_performance_tweaks_config.ini`.
- 🧠 **Smart Initialization**: Automatically waits (~40s timeout) for the game window to stabilize.
- ⏱️ **Zero-Latency Timer**: Forces **0.5ms** system timer resolution via Native `ntdll` API.
- 🧵 **Smart Core Scheduling**: 
    - Intelligent **Core 0 Bypass** to reduce OS-related micro-stutters.
    - Automatic **Affinity Masking** based on your CPU thread count.
- 🚀 **MMCSS Integration**: Registers game threads in the *Multimedia Class Scheduler Service* (Pro Audio/Games).
- 💾 **Memory Optimization**: Expands the process **Working Set** (512MB-2GB) to minimize I/O paging.
- ⚡ **Power Management**: Disables **Power Throttling** and prevents system sleep during gameplay.

### 🛡️ Anti-Cheat & Safety
> [!WARNING]
> **EASY ANTI-CHEAT MUST BE DISABLED.** 
> This tool modifies process memory and scheduling. Use it only in **OFFLINE MODE**. Using this online **WILL** result in a ban.

### 🛠️ Installation
1. **Prerequisites**: Ensure you have [Mod Engine 2/3](https://github.com/soulsmods/ModEngine2) or [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117) installed.
2. **Setup**:
    - Download `elden_ring_performance_tweaks.dll` and `er_performance_tweaks_config.ini`.
    - Place both files into your `ELDEN RING\Game\mods` directory.
3. **Verify**: Check `er_performance_tweaks_log.log` in the game folder after launch to confirm optimizations were applied.

---

<a name="russian"></a>

## ⚔️ Русский

Легковесная и мощная DLL для устранения "статтеров" и оптимизации работы движка **Elden Ring**.

> [!IMPORTANT]
> **Только для WINDOWS.** Мод использует низкоуровневые системные API Windows для управления памятью и планировщиком задач.

### ✨ Основные возможности
- ⚙️ **Полная настройка**: Управление всеми параметрами через `er_performance_tweaks_config.ini`.
- 🧠 **Умный запуск**: Автоматическое ожидание окна игры (таймаут 40с) для стабильной инициализации.
- ⏱️ **Таймер с нулевой задержкой**: Принудительная установка разрешения таймера **0.5мс** через Native API.
- 🧵 **Умное распределение ядер**: 
    - Пропуск **Ядра 0** для исключения влияния системных прерываний.
    - Автоматическая маска аффинити под ваше количество потоков.
- 🚀 **Интеграция MMCSS**: Регистрация потоков в системном планировщике мультимедиа (Pro Audio).
- 💾 **Оптимизация памяти**: Расширение **рабочего набора** (512МБ-2ГБ) для минимизации подгрузок с диска.
- ⚡ **Управление питанием**: Отключение **Power Throttling** и блокировка спящего режима во время игры.

### 🛡️ Античит и Безопасность
> [!WARNING]
> **EASY ANTI-CHEAT ДОЛЖЕН БЫТЬ ОТКЛЮЧЕН.** 
> Этот инструмент вмешивается в работу процесса. Используйте его только в **ОФФЛАЙН-РЕЖИМЕ**. Выход в онлайн с этим модом приведет к **БАНУ**.

### 🛠️ Установка
1. **Подготовка**: Убедитесь, что у вас установлен [Mod Engine 2/3](https://github.com/soulsmods/ModEngine2) или [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117).
2. **Настройка**:
    - Скачайте `elden_ring_performance_tweaks.dll` и `er_performance_tweaks_config.ini`.
    - Поместите оба файла в папку `ELDEN RING\Game\mods`.
3. **Проверка**: После запуска игры проверьте файл `er_performance_tweaks_log.log` для подтверждения работы твиков.

---

## 📜 License & Disclaimer
This project is licensed under the MIT License. Use at your own risk. Not affiliated with FromSoftware or Bandai Namco.

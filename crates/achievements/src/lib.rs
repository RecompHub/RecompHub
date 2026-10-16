//! Motor de conquistas: a rcheevos avalia, a cada frame, as condições no
//! formato do RetroAchievements lendo a memória por uma [`MemorySource`].
//!
//! Só a parte de runtime da rcheevos é usada: sem servidor, as definições
//! chegam como texto (`"0xH000318=5"`) e os desbloqueios voltam como eventos.

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_int, c_void};
use std::ptr::NonNull;

pub use recomphub_memory::MemorySource;

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    #[repr(C)]
    pub struct RcRuntime {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct RcRuntimeEvent {
        pub id: u32,
        pub value: i32,
        pub kind: u8,
    }

    pub type Peek = unsafe extern "C" fn(address: u32, num_bytes: u32, ud: *mut c_void) -> u32;
    pub type EventHandler = unsafe extern "C" fn(event: *const RcRuntimeEvent);

    // Valores de rc_runtime.h (v12.5.0).
    pub const EVENT_ACHIEVEMENT_ACTIVATED: u8 = 0;
    pub const EVENT_ACHIEVEMENT_PAUSED: u8 = 1;
    pub const EVENT_ACHIEVEMENT_RESET: u8 = 2;
    pub const EVENT_ACHIEVEMENT_TRIGGERED: u8 = 3;
    pub const EVENT_ACHIEVEMENT_PRIMED: u8 = 4;
    pub const EVENT_ACHIEVEMENT_DISABLED: u8 = 9;
    pub const EVENT_ACHIEVEMENT_UNPRIMED: u8 = 11;
    pub const EVENT_ACHIEVEMENT_PROGRESS_UPDATED: u8 = 12;

    unsafe extern "C" {
        pub fn rc_runtime_alloc() -> *mut RcRuntime;
        pub fn rc_runtime_destroy(runtime: *mut RcRuntime);
        pub fn rc_runtime_activate_achievement(
            runtime: *mut RcRuntime,
            id: u32,
            memaddr: *const c_char,
            unused_l: *mut c_void,
            unused_funcs_idx: c_int,
        ) -> c_int;
        pub fn rc_runtime_deactivate_achievement(runtime: *mut RcRuntime, id: u32);
        pub fn rc_runtime_do_frame(
            runtime: *mut RcRuntime,
            event_handler: EventHandler,
            peek: Peek,
            ud: *mut c_void,
            unused_l: *mut c_void,
        );
        pub fn rc_runtime_reset(runtime: *mut RcRuntime);
        pub fn rc_error_str(ret: c_int) -> *const c_char;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Condições ficaram falsas pela primeira vez; a conquista passa a valer.
    Activated(u32),
    Triggered(u32),
    Primed(u32),
    Unprimed(u32),
    Reset(u32),
    Paused(u32),
    /// Endereço inválido na definição.
    Disabled(u32),
    Progress(u32),
}

impl Event {
    fn from_ffi(event: &ffi::RcRuntimeEvent) -> Option<Self> {
        let id = event.id;
        Some(match event.kind {
            ffi::EVENT_ACHIEVEMENT_ACTIVATED => Event::Activated(id),
            ffi::EVENT_ACHIEVEMENT_TRIGGERED => Event::Triggered(id),
            ffi::EVENT_ACHIEVEMENT_PRIMED => Event::Primed(id),
            ffi::EVENT_ACHIEVEMENT_UNPRIMED => Event::Unprimed(id),
            ffi::EVENT_ACHIEVEMENT_RESET => Event::Reset(id),
            ffi::EVENT_ACHIEVEMENT_PAUSED => Event::Paused(id),
            ffi::EVENT_ACHIEVEMENT_DISABLED => Event::Disabled(id),
            ffi::EVENT_ACHIEVEMENT_PROGRESS_UPDATED => Event::Progress(id),
            _ => return None, // leaderboards ainda não são usados
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub code: i32,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for ParseError {}

thread_local! {
    // O handler de eventos da rcheevos não recebe userdata; os eventos de um
    // `do_frame` são coletados aqui (a chamada é síncrona, na mesma thread).
    static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
}

unsafe extern "C" fn on_event(event: *const ffi::RcRuntimeEvent) {
    if let Some(event) = unsafe { event.as_ref() }.and_then(Event::from_ffi) {
        EVENTS.with(|e| e.borrow_mut().push(event));
    }
}

/// Lê `num_bytes` (1, 2 ou 4) e monta o valor em little-endian, como o
/// `rc_libretro` faz com o buffer de memória dos emuladores.
unsafe extern "C" fn peek(address: u32, num_bytes: u32, ud: *mut c_void) -> u32 {
    let source = unsafe { &mut *ud.cast::<&mut dyn MemorySource>() };
    let mut buf = [0u8; 4];
    let len = (num_bytes as usize).min(4);
    if source.read(address, &mut buf[..len]) != len {
        return 0;
    }
    u32::from_le_bytes(buf)
}

pub struct Runtime {
    raw: NonNull<ffi::RcRuntime>,
}

// O rc_runtime_t é só dados; quem tem o `Runtime` tem acesso exclusivo.
unsafe impl Send for Runtime {}

impl Runtime {
    pub fn new() -> Self {
        let raw = NonNull::new(unsafe { ffi::rc_runtime_alloc() }).expect("rcheevos: sem memória");
        Self { raw }
    }

    /// Ativa uma conquista a partir das condições no formato do RA
    /// (ex.: `"0xH000318=5_0xH000319>0"`).
    pub fn activate(&mut self, id: u32, conditions: &str) -> Result<(), ParseError> {
        let conditions = CString::new(conditions).map_err(|_| ParseError {
            code: 0,
            message: "condição contém byte nulo".into(),
        })?;
        let code = unsafe {
            ffi::rc_runtime_activate_achievement(
                self.raw.as_ptr(),
                id,
                conditions.as_ptr(),
                std::ptr::null_mut(),
                0 as c_int,
            )
        };
        if code == 0 {
            return Ok(());
        }
        let message = unsafe { CStr::from_ptr(ffi::rc_error_str(code)) }.to_string_lossy().into_owned();
        Err(ParseError { code, message })
    }

    pub fn deactivate(&mut self, id: u32) {
        unsafe { ffi::rc_runtime_deactivate_achievement(self.raw.as_ptr(), id) };
    }

    /// Avalia todas as conquistas ativas uma vez ("um frame").
    pub fn do_frame(&mut self, memory: &mut dyn MemorySource) -> Vec<Event> {
        let mut source: &mut dyn MemorySource = memory;
        let ud = (&mut source as *mut &mut dyn MemorySource).cast::<c_void>();
        EVENTS.with(|e| e.borrow_mut().clear());
        unsafe { ffi::rc_runtime_do_frame(self.raw.as_ptr(), on_event, peek, ud, std::ptr::null_mut()) };
        EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
    }

    /// Volta todas as conquistas ao estado inicial (ex.: ao recarregar o jogo).
    pub fn reset(&mut self) {
        unsafe { ffi::rc_runtime_reset(self.raw.as_ptr()) };
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { ffi::rc_runtime_destroy(self.raw.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use recomphub_memory::BufferSource;

    fn ram() -> BufferSource {
        BufferSource(vec![0u8; 0x1000])
    }

    #[test]
    fn achievement_triggers_only_after_being_false_first() {
        let mut runtime = Runtime::new();
        // 8 bits em 0x000318 igual a 5.
        runtime.activate(1, "0xH000318=5").unwrap();

        let mut mem = ram();
        mem.0[0x318] = 5;
        // Verdadeira já no primeiro frame: a rcheevos espera ficar falsa antes
        // (evita desbloquear ao carregar um save).
        assert!(!runtime.do_frame(&mut mem).contains(&Event::Triggered(1)));

        mem.0[0x318] = 0;
        assert!(runtime.do_frame(&mut mem).contains(&Event::Activated(1)));

        mem.0[0x318] = 5;
        assert_eq!(runtime.do_frame(&mut mem), vec![Event::Triggered(1)]);

        // Depois de desbloqueada, não dispara de novo.
        assert!(runtime.do_frame(&mut mem).is_empty());
    }

    #[test]
    fn reads_16_and_32_bit_values_little_endian() {
        let mut runtime = Runtime::new();
        runtime.activate(2, "0x 000100=4660").unwrap(); // 16 bits == 0x1234
        runtime.activate(3, "0xX000200=305419896").unwrap(); // 32 bits == 0x12345678

        let mut mem = ram();
        runtime.do_frame(&mut mem);
        mem.0[0x100..0x102].copy_from_slice(&0x1234u16.to_le_bytes());
        mem.0[0x200..0x204].copy_from_slice(&0x1234_5678u32.to_le_bytes());

        let events = runtime.do_frame(&mut mem);
        assert!(events.contains(&Event::Triggered(2)), "{events:?}");
        assert!(events.contains(&Event::Triggered(3)), "{events:?}");
    }

    #[test]
    fn invalid_conditions_are_rejected() {
        let mut runtime = Runtime::new();
        let err = runtime.activate(4, "isto não é uma condição").unwrap_err();
        assert_ne!(err.code, 0);
        assert!(!err.message.is_empty());
    }
}

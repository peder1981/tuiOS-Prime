//! Driver de audio basico — Intel HDA (stub para Fase 3)

pub struct HdaAudio;

impl HdaAudio {
    pub fn probe(_dev: &crate::pci::PciDev) -> Option<Self> {
        // TODO: Implementar em Fase 3
        None
    }

    pub fn play_pcm(&self, _data: &[u8]) -> Result<(), &'static str> {
        Err("audio not implemented")
    }
}

pub fn init() {
    crate::println!("AUDIO: stub (Fase 3)");
}

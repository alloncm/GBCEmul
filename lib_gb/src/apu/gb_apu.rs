use super::channel::Channel;
use super::wave_sample_producer::WaveSampleProducer;
use super::audio_device::AudioDevice;
use crate::mmu::memory::Memory;
use crate::utils::bit_masks::*;

pub const AUDIO_BUFFER_SIZE:usize = 0x100;

pub struct GbApu<Device: AudioDevice>{
    pub wave_channel:Channel<WaveSampleProducer>,

    audio_buffer:[f32;AUDIO_BUFFER_SIZE],
    current_cycle:u32,
    device:Device
}

impl<Device: AudioDevice> GbApu<Device>{
    pub fn new(device: Device) -> Self {
        GbApu{
            wave_channel:Channel::<WaveSampleProducer>::new(),
            audio_buffer:[0.0; AUDIO_BUFFER_SIZE],
            current_cycle:0,
            device:device
        }
    }

    pub fn cycle(&mut self, memory:&mut dyn Memory, cycles_passed:u8){
        
        //add timer 
        for _ in 0..cycles_passed{   
            if self.current_cycle as usize >= AUDIO_BUFFER_SIZE{
                self.current_cycle = 0;
                self.device.push_buffer(&self.audio_buffer);
            }

            self.audio_buffer[self.current_cycle as usize] = self.wave_channel.get_audio_sample();

            self.current_cycle += 1;
        }
    }

    fn prepare_wave_channel(&mut self, memory:&dyn Memory){
        self.wave_channel.sound_length = memory.read(0xFF1B);
        self.wave_channel.enable = memory.read(0xFF1A) & BIT_7_MASK != 0;
        //I want bits 5-6
        self.wave_channel.sample_producer.volume = (memory.read(0xFF1C)>>5) & 0b011;
        let mut freq = memory.read(0xFF1D) as u16;
        let nr34 = memory.read(0xFF1E);
        freq |= ((nr34 & 0b111) as u16) << 8;
        self.wave_channel.frequency = freq;
        self.wave_channel.trigger = nr34 & BIT_7_MASK != 0;
        self.wave_channel.length_enable = nr34 & BIT_6_MASK != 0;

        for i in 0..=0xF{
            self.wave_channel.sample_producer.wave_samples[i] = memory.read(0xFF30 + i as u16);
        }
    }

    fn update_registers(&mut self, memory:&mut dyn Memory){
        memory.write(0xFF1B, self.wave_channel.sound_length);
    }
}

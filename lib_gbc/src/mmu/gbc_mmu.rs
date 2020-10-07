use super::memory::Memory;
use super::video_memory::ReadOnlyVideoMemory;
use super::ram::Ram;
use super::vram::VRam;
use super::io_ports::IoPorts;
use crate::utils::memory_registers::{
    DMA_REGISTER_ADDRESS,
    BOOT_REGISTER_ADDRESS
};
use super::carts::mbc::Mbc;
use crate::ppu::ppu_state::PpuState;
use std::boxed::Box;

pub const BOOT_ROM_SIZE:usize = 0x100;
const HRAM_SIZE:usize = 0x7F;
const SPRITE_ATTRIBUTE_TABLE_SIZE:usize = 0xA0;

const BAD_READ_VALUE:u8 = 0xFF;

pub struct GbcMmu<'a>{
    pub ram: Ram,
    pub vram: VRam,
    pub dma_trasfer_trigger:bool,
    pub finished_boot:bool,
    pub io_ports: IoPorts,
    boot_rom:[u8;BOOT_ROM_SIZE],
    mbc: &'a mut Box<dyn Mbc>,
    sprite_attribute_table:[u8;SPRITE_ATTRIBUTE_TABLE_SIZE],
    hram: [u8;HRAM_SIZE],
    interupt_enable_register:u8,
    pub ppu_state:PpuState
}


impl<'a> Memory for GbcMmu<'a>{
    fn read(&self, address:u16)->u8{
        return match address{
            0x0..=0xFF=>{
                if self.finished_boot{
                    return self.mbc.read_bank0(address);
                }
                
                return self.boot_rom[address as usize];
            },
            0x100..=0x3FFF=>self.mbc.read_bank0(address),
            0x4000..=0x7FFF=>self.mbc.read_current_bank(address-0x4000),
            0x8000..=0x9FFF=>{
                if self.is_vram_ready_for_io(){
                    return self.vram.read_current_bank(address-0x8000);
                }
                else{
                    return BAD_READ_VALUE;
                }
            },
            0xA000..=0xBFFF=>self.mbc.read_external_ram(address-0xA000),
            0xC000..=0xCFFF =>self.ram.read_bank0(address - 0xC000), 
            0xD000..=0xDFFF=>self.ram.read_current_bank(address-0xD000),
            0xE000..=0xFDFF=>self.ram.read_bank0(address - 0xE000),
            0xFE00..=0xFE9F=>{
                if self.is_oam_ready_for_io(){
                    return self.sprite_attribute_table[(address-0xFE00) as usize];
                }
                else{
                    return BAD_READ_VALUE;
                }
            },
            0xFEA0..=0xFEFF=>0x0,
            0xFF00..=0xFF7F=>self.io_ports.read(address - 0xFF00),
            0xFF80..=0xFFFE=>self.hram[(address-0xFF80) as usize],
            0xFFFF=>self.interupt_enable_register
        }
    }

    fn write(&mut self, address:u16, value:u8){
        if address == DMA_REGISTER_ADDRESS{
            self.dma_trasfer_trigger = true;
        }

        match address{
            0x0..=0x7FFF=>self.mbc.write_rom(address, value),
            0x8000..=0x9FFF=>{
                if self.is_vram_ready_for_io(){
                    self.vram.write_current_bank(address-0x8000, value);
                }
            },
            0xA000..=0xBFFF=>self.mbc.write_external_ram(address-0xA000,value),
            0xC000..=0xCFFF =>self.ram.write_bank0(address - 0xC000,value), 
            0xE000..=0xFDFF=>self.ram.write_bank0(address - 0xE000,value),
            0xD000..=0xDFFF=>self.ram.write_current_bank(address-0xD000,value),
            0xFE00..=0xFE9F=>{
                if self.is_oam_ready_for_io(){
                    self.sprite_attribute_table[(address-0xFE00) as usize] = value;
                }
            },
            0xFEA0..=0xFEFF=>{},
            0xFF00..=0xFF7F=>self.io_ports.write(address - 0xFF00, value),
            0xFF80..=0xFFFE=>self.hram[(address-0xFF80) as usize] = value,
            0xFFFF=>self.interupt_enable_register = value
        }
    }
}

impl<'a> ReadOnlyVideoMemory for GbcMmu<'a>{
    fn read(&self, address: u16)->u8{
        return match address{
            0x8000..=0x9FFF=>self.vram.read_current_bank(address - 0x8000),
            0xFE00..=0xFE9F=>self.sprite_attribute_table[(address - 0xFE00) as usize],
            _=>std::panic!("No one should read no video memory using this trait")
        }
    }
}

impl<'a> GbcMmu<'a>{
    pub fn new_with_bootrom(mbc:&'a mut Box<dyn Mbc>, boot_rom:[u8;BOOT_ROM_SIZE])->Self{
        GbcMmu{
            ram:Ram::default(),
            io_ports:IoPorts::default(),
            mbc:mbc,
            vram:VRam::default(),
            sprite_attribute_table:[0;SPRITE_ATTRIBUTE_TABLE_SIZE],
            hram:[0;HRAM_SIZE],
            interupt_enable_register:0,
            dma_trasfer_trigger:false,
            boot_rom:boot_rom,
            finished_boot:false,
            ppu_state:PpuState::OamSearch
        }
    }

    pub fn new(mbc:&'a mut Box<dyn Mbc>)->Self{
        let mut mmu = GbcMmu{
            ram:Ram::default(),
            io_ports:IoPorts::default(),
            mbc:mbc,
            vram:VRam::default(),
            sprite_attribute_table:[0;SPRITE_ATTRIBUTE_TABLE_SIZE],
            hram:[0;HRAM_SIZE],
            interupt_enable_register:0,
            dma_trasfer_trigger:false,
            boot_rom:[0;BOOT_ROM_SIZE],
            finished_boot:true,
            ppu_state:PpuState::OamSearch
        };

        //Setting the bootrom register to be set (the boot sequence has over)
        mmu.io_ports.write_unprotected(BOOT_REGISTER_ADDRESS - 0xFF00, 1);
        
        mmu
    }

    fn is_oam_ready_for_io(&self)->bool{
        return true;
        //TODO: uncomment when cycle accureate
        //let ppu_state = self.ppu_state as u8;
        //return ppu_state != PpuState::OamSearch as u8 && ppu_state != PpuState::PixelTransfer as u8
    }

    fn is_vram_ready_for_io(&self)->bool{
        return true;
        //TODO: uncomment when cycle accureate
        //return self.ppu_state as u8 != PpuState::PixelTransfer as u8;
    }
}
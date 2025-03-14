//! Contains helper functions that interact with the Windows OS through API calls.

#![warn(missing_docs)]


use core::ffi::c_void;
use std::collections::HashMap;
use std::mem::transmute;
use std::ptr;
use std::thread;
use std::time::Duration;

use log::{debug, info};

use windows::core::{
    Error,
    HRESULT,
    HSTRING, 
    Result,
};
use windows::Win32::Foundation::{
    CloseHandle,
    FARPROC,
    HANDLE,
    HMODULE,
    MAX_PATH,
};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{
    GetModuleHandleA,
    GetProcAddress,
};
use windows::Win32::System::Memory::{
    MEM_COMMIT,
    MEM_RELEASE,
    MEM_RESERVE,
    PAGE_READWRITE,
    VirtualAllocEx,
    VirtualFreeEx,
};
use windows::Win32::System::ProcessStatus::{
    EnumProcessModules,
    EnumProcesses,
    GetModuleBaseNameW,
};
use windows::Win32::System::Threading::{
    CreateRemoteThread,
    OpenProcess,
    PROCESS_ALL_ACCESS,
};
use windows_strings::s;


/// Returns the base address of the `LoadLibraryW` WinAPI function.
fn get_load_library_w_handle() -> Result<FARPROC> {
    let h_kernel32: HMODULE = unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlea.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/LibraryLoader/fn.GetModuleHandleA.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/GetModuleHandleA.
        GetModuleHandleA(s!("kernel32.dll"))
    }?;

    let p_address: FARPROC = unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/LibraryLoader/fn.GetProcAddress.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/GetProcAddress.
        GetProcAddress(h_kernel32, s!("LoadLibraryW"))
    };
    Ok(p_address)
}

/// Returns a vector containg the PIDs of all running processes.
fn get_process_ids() -> Result<Vec<u32>> {
    let mut vec_capacity: usize = 1024;
    let mut process_ids = Vec::with_capacity(vec_capacity);

    let mut cb_needed: u32 = 0;
    for _ in 0..3 {
        process_ids.resize(vec_capacity, 0);

        let _success = unsafe { 
            EnumProcesses(
                process_ids.as_mut_ptr(),
                process_ids.len().try_into()?,
                &mut cb_needed,
            )
        };

        if cb_needed as usize != process_ids.len() {
            process_ids.retain(|&i| i != 0);
            return Ok(process_ids);
        }

        debug!("buffer passed to EnumProcesses is too small ({})", vec_capacity);
        vec_capacity *= 2;
    };

    Err(Error::new(HRESULT(-1), "Maximum amount of reallocations reached"))
}


/// Returns a hashmap that maps process names to their respective PIDs.
/// 
/// The hashmap does NOT contain all name/pid associations, but only the ones of processes
/// to which a handle with `PROCESS_ALL_ACCESS` permissions can be opened.
pub fn get_process_name_pid_mapping() -> Result<HashMap<String, u32>> {
    let mut name_and_pid: HashMap<String, u32> = HashMap::new();
    let process_ids = get_process_ids()?;

    for pid in &process_ids {
        let Ok(h_process) = (unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess.
            // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.OpenProcess.html.
            // https://microsoft.github.io/windows-rs/features/#/latest/search/OpenProcess.
            OpenProcess(
                PROCESS_ALL_ACCESS,
                false,
                *pid,
            )
        }) else { continue };

        let mut h_module: HMODULE = Default::default();
        let mut dw_return_len: u32 = 0;
            
        if unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-enumprocessmodules.
            // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/ProcessStatus/fn.EnumProcesses.html.
            // https://microsoft.github.io/windows-rs/features/#/latest/search/EnumProcessModules.
            EnumProcessModules(
                h_process,
                &mut h_module,
                std::mem::size_of::<HMODULE>().try_into().unwrap(),
                &mut dw_return_len,
            )
        }.is_err() {
            continue;
        }

        let mut module_base_name_w: [u16; MAX_PATH as usize] = [0; MAX_PATH as usize];
        if unsafe {
            // https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getmodulebasenamew.
            // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/ProcessStatus/fn.GetModuleBaseNameW.html.
            // https://microsoft.github.io/windows-rs/features/#/latest/search/GetModuleBaseNameW.
            GetModuleBaseNameW(
                h_process,
                Some(h_module),
                &mut module_base_name_w,
            )
        } == 0 {
            continue
        };

        let module_base_name_h: HSTRING = HSTRING::from_wide(&module_base_name_w);
        name_and_pid.insert(module_base_name_h.to_string().trim_matches(char::from(0)).to_owned(), *pid);
    }

    Ok(name_and_pid)
}


/// Returns the PID of a process given its name.
pub fn find_process_by_name(name: &str, case_insensitive: Option<bool>) -> Result<u32> {
    let case_insensitive: bool = case_insensitive.unwrap_or(false);

    for (key, value) in &get_process_name_pid_mapping()? {
        if name == key {
            return Ok(*value);
        }
        if case_insensitive && name.to_uppercase() == key.to_uppercase() {
            return Ok(*value);
        }
    }
    
    Err(Error::new(HRESULT(-1), format!("process {:#} not found", name)))
}


/// Loads a DLL into a target process.
pub fn load_dll(pid: u32, dll_path: &str) -> Result<()> {
    let dll_path_w: HSTRING = HSTRING::from(dll_path);
    let dw_size_to_write: usize = dll_path_w.len() * 2 + 1; // 2 bytes per character + \0.

    // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw.
    let p_load_library_w: *mut c_void = unsafe {
        transmute(get_load_library_w_handle()?)
    };
    debug!("LoadLibraryW address: {:#x}", p_load_library_w as isize);

    let h_process: HANDLE = unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.OpenProcess.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/OpenProcess.
        OpenProcess(
            PROCESS_ALL_ACCESS,
            false,
            pid,
        )?
    };
    debug!("target process handle: {:?}", h_process);

    let p_address: *mut c_void = unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualallocex.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Memory/fn.VirtualAllocEx.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/VirtualAllocEx.
        VirtualAllocEx(
            h_process,
            None,
            dw_size_to_write,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    if p_address.is_null() {
        return Err(Error::from_win32());
    }
    debug!("address of externally allocated memory: {:?}", p_address.clone());

    let mut lp_number_of_bytes_written: usize = 0;
    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Diagnostics/Debug/fn.WriteProcessMemory.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/WriteProcessMemory.
        WriteProcessMemory(
            h_process,
            p_address,
            dll_path_w.as_ptr() as *const c_void,
            dw_size_to_write,
            Some(&mut lp_number_of_bytes_written),
        )
    }?;

    if lp_number_of_bytes_written != dw_size_to_write {
        return Err(Error::new(HRESULT(-1), "failed to write the DLL path in memory"));
    }

    let h_thread = unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createremotethread.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/fn.CreateRemoteThread.html.
        // https://microsoft.github.io/windows-rs/features/#/latestsearch/CreateRemoteThread.
        CreateRemoteThread(
            h_process,
            None, 
            0,
            Some(transmute(p_load_library_w)),
            Some(p_address),
            0,
            None,
        )
    }?;

    if h_thread == HANDLE(ptr::null_mut()) {
        return Err(Error::from_win32());
    }
    info!("{}", format!("remote thread started in process ({}): {:?}", pid, h_thread));

    thread::sleep(Duration::from_millis(400));

    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Diagnostics/Debug/fn.WriteProcessMemory.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/WriteProcessMemory.
        WriteProcessMemory(
            h_process,
            p_address,
            vec![0; dw_size_to_write].as_ptr() as *const c_void,
            dw_size_to_write,
            Some(&mut lp_number_of_bytes_written),
        )
    }?;

    if lp_number_of_bytes_written != dw_size_to_write {
        return Err(Error::new(HRESULT(-1), "failed to zero out the allocated memory"));
    }

    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualfreeex.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Memory/fn.VirtualFreeEx.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/VirtualFreeEx.
        VirtualFreeEx(
            h_process,
            p_address,
            0,
            MEM_RELEASE,
        )
    }?;
    debug!("releasing the allocated memory");

    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle.
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Foundation/fn.CloseHandle.html.
        // https://microsoft.github.io/windows-rs/features/#/latest/search/CloseHandle.
        CloseHandle(h_thread)?;
        CloseHandle(h_process)?;
    }

    Ok(())
}
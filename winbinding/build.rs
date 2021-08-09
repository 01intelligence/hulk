fn main() {
    windows::build! {
        Windows::Win32::System::WindowsProgramming::{
            DRIVE_FIXED,
            DRIVE_REMOVABLE,
            DRIVE_REMOTE,
            DRIVE_RAMDISK,
        },
        Windows::Win32::Storage::FileSystem::{
            GetDiskFreeSpaceW,
            GetDiskFreeSpaceExW,
            GetVolumeInformationW,
            GetVolumePathNameW,
            GetDriveTypeW,
        },
    };
}

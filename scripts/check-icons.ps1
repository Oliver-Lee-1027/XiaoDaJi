param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [string]$Icon = (Join-Path (Split-Path $PSScriptRoot -Parent) 'assets\xiaodaji.ico')
)
$ErrorActionPreference = 'Stop'
# Read PE resources as data; never execute the program being checked or use Explorer's cache.
if (-not ('XiaoDaJiIconResources' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.IO;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class XiaoDaJiIconResources {
    delegate bool ResourceName(IntPtr module, IntPtr type, IntPtr name, IntPtr parameter);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr LoadLibraryEx(string file, IntPtr reserved, uint flags);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern bool EnumResourceNames(IntPtr module, IntPtr type, ResourceName callback, IntPtr parameter);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll")] static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")] static extern uint SizeofResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr resource);
    [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
    static byte[] Read(IntPtr module, IntPtr type, IntPtr name) {
        IntPtr resource = FindResource(module, name, type);
        if (resource == IntPtr.Zero) throw new InvalidDataException("Missing icon resource");
        int length = checked((int)SizeofResource(module, resource));
        byte[] result = new byte[length];
        Marshal.Copy(LockResource(LoadResource(module, resource)), result, 0, length);
        return result;
    }
    public static int Verify(string executable, string icon) {
        byte[] expected = File.ReadAllBytes(icon);
        int count = BitConverter.ToUInt16(expected, 4);
        IntPtr module = LoadLibraryEx(executable, IntPtr.Zero, 0x22);
        if (module == IntPtr.Zero) throw new InvalidDataException("Cannot load executable resources");
        try {
            var groups = new List<byte[]>();
            ResourceName callback = delegate(IntPtr m, IntPtr t, IntPtr n, IntPtr p) {
                groups.Add(Read(m, t, n)); return true;
            };
            EnumResourceNames(module, new IntPtr(14), callback, IntPtr.Zero);
            GC.KeepAlive(callback);
            if (groups.Count == 0) throw new InvalidDataException("Executable has no icon group");
            foreach (byte[] group in groups) {
                if (BitConverter.ToUInt16(group, 4) != count) throw new InvalidDataException("Icon frame count mismatch");
                for (int i=0; i<count; i++) {
                    int a = 6 + i*16, b = 6 + i*14;
                    // rc.exe normalizes planes/bit-count; dimensions and pixel bytes must still match.
                    for (int j=0; j<4; j++) {
                        if (expected[a+j] != group[b+j]) throw new InvalidDataException("Icon directory mismatch, frame " + i + ", byte " + j + ": expected " + expected[a+j] + ", actual " + group[b+j]);
                    }
                    int length = checked((int)BitConverter.ToUInt32(expected, a+8));
                    int offset = checked((int)BitConverter.ToUInt32(expected, a+12));
                    byte[] actual = Read(module, new IntPtr(3), new IntPtr(BitConverter.ToUInt16(group, b+12)));
                    if (actual.Length != length) throw new InvalidDataException("Icon image size mismatch");
                    for (int j=0; j<length; j++) {
                        if (actual[j] != expected[offset+j]) throw new InvalidDataException("Icon pixels mismatch, frame " + i);
                    }
                }
            }
            return groups.Count;
        } finally { FreeLibrary(module); }
    }
}
"@
}
$file = (Get-Item -LiteralPath $Executable).FullName
$count = [XiaoDaJiIconResources]::Verify($file, (Get-Item -LiteralPath $Icon).FullName)
Write-Output "PASS: $file embeds the fox icon ($count group, all 9 sizes verified byte-for-byte)."
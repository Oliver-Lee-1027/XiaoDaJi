// WScript.Shell can return ANSI target paths. Use the Unicode Shell interface.
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;

namespace XiaoDaJi
{
    public sealed class ShortcutInfo
    {
        public string TargetPath { get; internal set; }
        public string Arguments { get; internal set; }
        public string WorkingDirectory { get; internal set; }
    }

    public static class WindowsShortcut
    {
        // SLGP_RAWPATH: read the stored target; never resolve or launch it.
        private const uint RawPath = 4;
        private const int PathCapacity = 32768;

        public static ShortcutInfo Read(string shortcutPath)
        {
            object instance = new ShellLink();
            try
            {
                ((IPersistFile)instance).Load(Path.GetFullPath(shortcutPath), 0);
                IShellLinkW link = (IShellLinkW)instance;
                StringBuilder target = new StringBuilder(PathCapacity);
                StringBuilder arguments = new StringBuilder(PathCapacity);
                StringBuilder directory = new StringBuilder(PathCapacity);
                link.GetPath(target, target.Capacity, IntPtr.Zero, RawPath);
                link.GetArguments(arguments, arguments.Capacity);
                link.GetWorkingDirectory(directory, directory.Capacity);
                return new ShortcutInfo {
                    TargetPath = target.ToString(),
                    Arguments = arguments.ToString(),
                    WorkingDirectory = directory.ToString()
                };
            }
            finally
            {
                Marshal.FinalReleaseComObject(instance);
            }
        }

        // Used by regression fixtures, including under an English process code page.
        public static void Write(string shortcutPath, string targetPath, string arguments)
        {
            object instance = new ShellLink();
            try
            {
                string target = Path.GetFullPath(targetPath);
                IShellLinkW link = (IShellLinkW)instance;
                link.SetPath(target);
                link.SetArguments(arguments ?? String.Empty);
                link.SetWorkingDirectory(Path.GetDirectoryName(target));
                link.SetIconLocation(target, 0);
                ((IPersistFile)instance).Save(Path.GetFullPath(shortcutPath), true);
            }
            finally
            {
                Marshal.FinalReleaseComObject(instance);
            }
        }

        [ComImport, Guid("00021401-0000-0000-C000-000000000046")]
        private class ShellLink { }

        // Preserve the complete native vtable, including unused methods.
        [ComImport, Guid("000214F9-0000-0000-C000-000000000046")]
        [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        private interface IShellLinkW
        {
            void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder path, int capacity, IntPtr findData, uint flags);
            void GetIDList(out IntPtr idList);
            void SetIDList(IntPtr idList);
            void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder description, int capacity);
            void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string description);
            void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder directory, int capacity);
            void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string directory);
            void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder arguments, int capacity);
            void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string arguments);
            void GetHotkey(out ushort hotkey);
            void SetHotkey(ushort hotkey);
            void GetShowCmd(out int showCommand);
            void SetShowCmd(int showCommand);
            void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder iconPath, int capacity, out int index);
            void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string iconPath, int index);
            void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string path, uint reserved);
            void Resolve(IntPtr window, uint flags);
            void SetPath([MarshalAs(UnmanagedType.LPWStr)] string path);
        }
    }
}

using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using XiaoDaJi;

class ShortcutUnicodeTest
{
    [DllImport("kernel32.dll")]
    private static extern uint GetACP();

    private static void Require(bool success, string message)
    {
        if (!success) throw new Exception(message);
    }

    [STAThread]
    private static int Main(string[] args)
    {
        Console.OutputEncoding = System.Text.Encoding.UTF8;
        try
        {
            string root = args[0];
            string directory = Path.Combine(root, "installed \u4e2d\u6587 files");
            Directory.CreateDirectory(directory);
            string target = Path.Combine(directory, "\u5c0f\u59b2\u5df1.exe");
            File.WriteAllText(target, "test-only executable placeholder");
            string arguments = "--settings --name=\u4e2d\u6587";
            string asciiLink = Path.Combine(root, "shortcut.lnk");
            string chineseLink = Path.Combine(directory, "\u4e2d\u6587\u5feb\u6377\u65b9\u5f0f.lnk");
            foreach (string path in new string[] { asciiLink, chineseLink })
            {
                WindowsShortcut.Write(path, target, arguments);
                ShortcutInfo actual = WindowsShortcut.Read(path);
                Require(actual.TargetPath == target, "Unicode target path did not round-trip");
                Require(File.Exists(actual.TargetPath), "Unicode target does not exist");
                Require(actual.Arguments == arguments, "Unicode arguments did not round-trip");
                Require(actual.WorkingDirectory == directory, "Unicode working directory did not round-trip");
            }
            string other = Path.Combine(directory, "other.exe");
            WindowsShortcut.Write(asciiLink, other, "");
            Require(WindowsShortcut.Read(asciiLink).TargetPath != target, "Wrong target was accepted");
            bool rejected = false;
            string invalid = Path.Combine(root, "invalid.lnk");
            File.WriteAllText(invalid, "not a shortcut");
            try { WindowsShortcut.Read(invalid); } catch (COMException) { rejected = true; }
            Require(rejected, "Invalid shortcut was accepted");
            WindowsShortcut.Write(asciiLink, target, arguments);

            // Diagnostic only: a future WScript fix must not break this test.
            object shell = Activator.CreateInstance(Type.GetTypeFromProgID("WScript.Shell"));
            object legacy = null;
            try
            {
                legacy = shell.GetType().InvokeMember("CreateShortcut", BindingFlags.InvokeMethod,
                    null, shell, new object[] { asciiLink });
                string ansiTarget = (string)legacy.GetType().InvokeMember("TargetPath", BindingFlags.GetProperty,
                    null, legacy, null);
                Console.WriteLine("ACP=" + GetACP() + "; WScript ANSI mismatch=" + (ansiTarget != target));
            }
            finally
            {
                if (legacy != null) Marshal.FinalReleaseComObject(legacy);
                Marshal.FinalReleaseComObject(shell);
            }
            Console.WriteLine("PASS: Unicode shortcut names, targets, arguments, working directories and invalid-target checks");
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error);
            return 1;
        }
    }
}

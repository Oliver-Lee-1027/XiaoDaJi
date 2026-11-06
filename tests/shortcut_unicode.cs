using System;
using System.ComponentModel;
using System.Text;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using XiaoDaJi;

class ShortcutUnicodeTest
{
    [DllImport("kernel32.dll")]
    private static extern uint GetACP();

    [DllImport("kernel32.dll", EntryPoint = "GetLongPathNameW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetLongPathName(string path, StringBuilder result, uint capacity);

    [DllImport("kernel32.dll", EntryPoint = "GetShortPathNameW", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint GetShortPathName(string path, StringBuilder result, uint capacity);

    private static string LongPath(string path)
    {
        StringBuilder result = new StringBuilder(32768);
        uint length = GetLongPathName(Path.GetFullPath(path), result, (uint)result.Capacity);
        if (length == 0) throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot expand existing path: " + path);
        if (length >= result.Capacity) throw new IOException("Path normalization buffer is too small: " + path);
        return result.ToString();
    }

    private static bool SameExistingPath(string first, string second)
    {
        // Shell links may expand 8.3 aliases (e.g. RUNNER~1) and normalize case.
        // Both paths must exist and normalize to the entire same Unicode path.
        if (!(File.Exists(first) || Directory.Exists(first)) ||
            !(File.Exists(second) || Directory.Exists(second))) return false;
        return String.Equals(LongPath(first), LongPath(second), StringComparison.OrdinalIgnoreCase);
    }

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
            string style = args.Length > 1 ? args[1] : "long";
            string longRoot = LongPath(root);
            if (style == "short")
            {
                StringBuilder shortRoot = new StringBuilder(32768);
                uint length = GetShortPathName(longRoot, shortRoot, (uint)shortRoot.Capacity);
                if (length == 0) throw new Win32Exception(Marshal.GetLastWin32Error(), "Cannot query 8.3 path");
                if (length >= shortRoot.Capacity) throw new IOException("Short path buffer is too small");
                root = shortRoot.ToString();
                if (String.Equals(root, longRoot, StringComparison.OrdinalIgnoreCase))
                    Console.WriteLine("SKIP: distinct 8.3 alias unavailable on this volume; still testing the long path");
            }
            Console.WriteLine("ACP=" + GetACP() + "; style=" + style + "; input=[" + root + "]; long=[" + longRoot + "]");
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
                Require(SameExistingPath(actual.TargetPath, target),
                    "Unicode target path did not round-trip; expected=[" + target + "]; actual=[" + actual.TargetPath + "]");
                Require(File.Exists(actual.TargetPath), "Unicode target does not exist");
                Require(actual.Arguments == arguments, "Unicode arguments did not round-trip");
                Require(SameExistingPath(actual.WorkingDirectory, directory),
                    "Unicode working directory did not round-trip; expected=[" + directory + "]; actual=[" + actual.WorkingDirectory + "]");
            }
            string other = Path.Combine(directory, "other.exe");
            File.WriteAllText(other, "different target must be rejected");
            WindowsShortcut.Write(asciiLink, other, "");
            Require(!SameExistingPath(WindowsShortcut.Read(asciiLink).TargetPath, target), "Wrong target was accepted");
            string lossyTarget = target.Replace("\u5c0f\u59b2\u5df1", "???");
            Require(!SameExistingPath(lossyTarget, target), "Lossy Unicode target was accepted");
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
                Console.WriteLine("ACP=" + GetACP() + "; WScript ANSI mismatch=" + (!SameExistingPath(ansiTarget, target)));
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

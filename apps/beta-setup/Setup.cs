// Per-user, transactional beta setup. Downloads run only after an explicit setup action.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;
using Microsoft.Win32;

[assembly: AssemblyTitle("Dungeon World AI Beta Setup")]
[assembly: AssemblyProduct("Dungeon World AI Beta")]
[assembly: AssemblyVersion("0.2.1.0")]
namespace DungeonWorldAI.Setup {
    public class Installed { public string product; public string version; public string directory; public string productCode; }
    static class Program {
        internal const string ProductId = "ai.dungeonworld.beta";
        internal static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = 16000000 };
        static readonly object LogLock = new object();
        internal static readonly string Self = Assembly.GetExecutingAssembly().Location;
        internal static readonly string Local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        internal static readonly string TestRoot = Environment.GetEnvironmentVariable("DWA_SETUP_TEST_ROOT");
        internal static bool Testing { get { return !string.IsNullOrEmpty(TestRoot); } }
        internal static string Root { get { return Path.GetFullPath(Testing ? TestRoot : File.Exists(Path.Combine(Path.GetDirectoryName(Self), "installed.json")) ? Path.GetDirectoryName(Self) : Path.Combine(Local, "Programs", "Dungeon World AI Beta")); } }
        internal static string Data { get { return Testing ? Path.Combine(Root, "test-data") : Path.Combine(Local, "DungeonWorldAI"); } }
        internal static string StatePath { get { return Path.Combine(Root, "installed.json"); } }
        internal static string Model { get { return Path.Combine(Data, "models", "dm", "Qwen3-4B-Q4_K_M.gguf"); } }
        internal static string Python { get { return Path.Combine(Data, "runtimes", "art-env", "Scripts", "python.exe"); } }
        internal static string Log { get { return Path.Combine(Data, "Beta", "setup.log"); } }
        [STAThread] static int Main(string[] args) {
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            Application.EnableVisualStyles(); Application.SetCompatibleTextRenderingDefault(false);
            try {
                if (args.Length > 0 && args[0] == "/self-test") return SelfTest();
                if (args.Length > 0 && args[0] == "/play") { Play(); return 0; }
                if (args.Length > 0 && args[0] == "/uninstall") {
                    var state = ReadInstalled(); Guid code;
                    if (!Guid.TryParse(state.productCode, out code)) throw new Exception("Use Windows Settings to uninstall Dungeon World AI Beta.");
                    Process.Start("msiexec.exe", "/x " + code.ToString("B")); return 0;
                }
                bool configure = true;
                Application.Run(new SetupForm(configure)); return 0;
            } catch (Exception e) { WriteLog(e.ToString()); if (args.Any(a => a == "/self-test")) { Console.Error.WriteLine(e.ToString()); return 1; } MessageBox.Show(e.Message + "\n\nYour saved campaigns have not been removed.", "Dungeon World AI", MessageBoxButtons.OK, MessageBoxIcon.Information); return 1; }
        }
        internal static void WriteLog(string s) { lock (LogLock) { try { Directory.CreateDirectory(Path.GetDirectoryName(Log)); File.AppendAllText(Log, DateTime.UtcNow.ToString("u") + " " + s + Environment.NewLine, new UTF8Encoding(false)); } catch { } } }
        internal static string Hash(Stream s) { using (var h = SHA256.Create()) return BitConverter.ToString(h.ComputeHash(s)).Replace("-", "").ToLowerInvariant(); }
        internal static string HashFile(string path) { using (var f = File.OpenRead(path)) return Hash(f); }
        internal static string Child(string parent, string path) {
            if (string.IsNullOrWhiteSpace(path) || Path.IsPathRooted(path) || path.Contains(":") || path.Split('/', '\\').Any(s => s == ".." || s == "." || s.Length == 0)) throw new Exception("The setup package contains an invalid path.");
            string root = Path.GetFullPath(parent).TrimEnd('\\') + "\\";
            string full = Path.GetFullPath(Path.Combine(parent, path.Replace('/', '\\')));
            if (!full.StartsWith(root, StringComparison.OrdinalIgnoreCase)) throw new Exception("The setup package path is outside its folder.");
            string scan = Path.GetDirectoryName(full);
            while (!string.IsNullOrEmpty(scan) && scan.StartsWith(root.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase)) {
                if (Directory.Exists(scan) && (File.GetAttributes(scan) & FileAttributes.ReparsePoint) != 0) throw new Exception("Choose a normal local folder for setup.");
                scan = Path.GetDirectoryName(scan);
            }
            return full;
        }
        internal static Installed ReadInstalled() {
            if (!File.Exists(StatePath)) throw new Exception("Dungeon World AI is not installed yet. Open the beta installer first.");
            var s = Json.Deserialize<Installed>(File.ReadAllText(StatePath));
            if (s.product != ProductId || s.directory != "app") throw new Exception("The installed app record is invalid. Run the beta installer to repair it.");
            Child(Root, s.directory); return s;
        }
        internal static string AppFolder() { return Child(Root, ReadInstalled().directory); }
        internal static void AtomicJson(string file, object data) {
            string temp = file + "." + Guid.NewGuid().ToString("N") + ".tmp";
            File.WriteAllText(temp, Json.Serialize(data), new UTF8Encoding(false));
            if (!MoveFileEx(temp, file, 0x1 | 0x8)) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "The app record could not be updated. Close the app and choose Retry.");
        }
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern bool MoveFileEx(string source, string destination, uint flags);
        internal static void Play() {
            if (!HasWebView()) { Application.Run(new SetupForm(true)); return; }
            string folder = AppFolder(), app = Path.Combine(folder, "dungeon-world-ai-desktop.exe"); if (!File.Exists(app)) throw new Exception("The app files are missing. Run the beta installer again to repair them.");
            var start = new ProcessStartInfo(app) { WorkingDirectory = folder, UseShellExecute = false };
            start.EnvironmentVariables["DWA_DATA_DIR"] = Path.Combine(Data, "Beta", "saved-games");
            start.EnvironmentVariables["LOCAL_LLM_SERVER_PATH"] = Path.Combine(folder, "ai", "llama-server.exe"); start.EnvironmentVariables["LOCAL_LLM_MODEL_PATH"] = Model;
            start.EnvironmentVariables["DWA_DM_GPU_LAYERS"] = "0";
            start.EnvironmentVariables["DWA_ART_PYTHON_PATH"] = Python; start.EnvironmentVariables["DWA_ART_MODEL_DIR"] = Path.Combine(Data, "models", "sd-turbo");
            start.EnvironmentVariables["DWA_BETA_SETUP"] = Path.Combine(Root, "DungeonWorldAI.exe");
            Process.Start(start);
        }
        internal static bool HasWebView() {
            foreach (var hive in new[] { RegistryHive.CurrentUser, RegistryHive.LocalMachine }) foreach (var view in new[] { RegistryView.Registry32, RegistryView.Registry64 }) {
                using (var registry = RegistryKey.OpenBaseKey(hive, view)) using (var clients = registry.OpenSubKey(@"Software\Microsoft\EdgeUpdate\Clients")) {
                    if (clients == null) continue;
                    foreach (string name in clients.GetSubKeyNames()) using (var client = clients.OpenSubKey(name)) {
                        string product = Convert.ToString(client.GetValue("name")), version = Convert.ToString(client.GetValue("pv"));
                        if (product.IndexOf("WebView2", StringComparison.OrdinalIgnoreCase) >= 0 && !string.IsNullOrEmpty(version) && version != "0.0.0.0") return true;
                    }
                }
            }
            return false;
        }
        internal static async Task EnsureWebView(Action<int, string> progress, CancellationToken token) {
            if (HasWebView()) return;
            progress(0, "Adding Microsoft's WebView2 display component...");
            string temp = Path.Combine(Path.GetTempPath(), "DWA-WebView-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(temp);
            string file = Path.Combine(temp, "MicrosoftEdgeWebview2Setup.exe");
            try {
                using (var client = new HttpClient { Timeout = TimeSpan.FromMinutes(3) }) using (var response = await client.GetAsync("https://go.microsoft.com/fwlink/p/?LinkId=2124703", HttpCompletionOption.ResponseHeadersRead, token)) {
                    response.EnsureSuccessStatusCode();
                    using (var input = await response.Content.ReadAsStreamAsync()) using (var output = File.Create(file)) {
                        byte[] buffer = new byte[65536]; int count; long total = 0;
                        while ((count = await input.ReadAsync(buffer, 0, buffer.Length, token)) > 0) { total += count; if (total > 16 * 1024 * 1024) throw new Exception("Microsoft's display component download was unexpected. Choose Retry later."); await output.WriteAsync(buffer, 0, count, token); }
                    }
                }
                if (!Authenticode.IsMicrosoft(file)) throw new Exception("The Microsoft display component did not pass its signature check. Choose Retry later.");
                using (var p = Process.Start(new ProcessStartInfo(file, "/silent /install") { UseShellExecute = false, CreateNoWindow = true })) {
                    await Task.Run(() => p.WaitForExit());
                    if (!HasWebView()) throw new Exception("Microsoft WebView2 did not finish installing. Restart Windows, then run this setup again.");
                }
            } finally { try { DeleteOwnedTree(temp, Path.GetTempPath()); } catch { } }
        }
        internal static async Task Narrator(Action<int, string> progress, CancellationToken token) {
            const string hash = "7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5";
            const string url = "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf";
            Directory.CreateDirectory(Path.GetDirectoryName(Model));
            progress(0, "Checking the local Dungeon Master model...");
            if (File.Exists(Model) && HashFile(Model) == hash) { progress(100, "Local Dungeon Master ready."); return; }
            string partial = Model + ".download"; long have = File.Exists(partial) ? new FileInfo(partial).Length : 0;
            using (var handler = new HttpClientHandler { AllowAutoRedirect = true }) using (var client = new HttpClient(handler) { Timeout = Timeout.InfiniteTimeSpan }) {
                var request = new HttpRequestMessage(HttpMethod.Get, url); if (have > 0) request.Headers.Range = new System.Net.Http.Headers.RangeHeaderValue(have, null);
                using (var reply = await client.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, token)) {
                    if (reply.StatusCode == HttpStatusCode.RequestedRangeNotSatisfiable) { File.Delete(partial); throw new Exception("The saved partial model needs a fresh download. Choose Retry."); }
                    reply.EnsureSuccessStatusCode();
                    if (reply.StatusCode != HttpStatusCode.PartialContent) have = 0;
                    if (have > 0 && (reply.Content.Headers.ContentRange == null || reply.Content.Headers.ContentRange.From != have)) throw new Exception("The model server could not resume this download. Choose Retry later.");
                    long total = have + (reply.Content.Headers.ContentLength ?? 0);
                    if (total > 0 && new DriveInfo(Path.GetPathRoot(Model)).AvailableFreeSpace < total - have + 100L * 1024 * 1024) throw new Exception("The Dungeon Master needs about 3 GB of free disk space. Free some space and choose Retry.");
                    using (var input = await reply.Content.ReadAsStreamAsync()) using (var output = new FileStream(partial, have > 0 ? FileMode.Append : FileMode.Create, FileAccess.Write, FileShare.Read)) {
                        byte[] buffer = new byte[1024 * 1024]; int n; long done = have;
                        while ((n = await input.ReadAsync(buffer, 0, buffer.Length, token)) > 0) { await output.WriteAsync(buffer, 0, n, token); done += n; progress(total > 0 ? (int)Math.Min(98, done * 98 / total) : 0, "Downloading local Dungeon Master: " + (done / 1048576) + (total > 0 ? " / " + (total / 1048576) : "") + " MB"); }
                    }
                }
            }
            token.ThrowIfCancellationRequested(); progress(99, "Verifying the Dungeon Master download...");
            if (HashFile(partial) != hash) { File.Delete(partial); throw new Exception("The model download did not pass its integrity check. Choose Retry to download it again."); }
            if (!MoveFileEx(partial, Model, 0x1 | 0x8)) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "The verified model could not be saved. Close the app and choose Retry.");
            progress(100, "Local Dungeon Master ready.");
        }
        internal static async Task Art(Action<int, string> progress, CancellationToken token) {
            string basePython = Path.Combine(AppFolder(), "python", "python.exe");
            string environment = Path.Combine(Data, "runtimes", "art-env");
            progress(0, "Preparing a separate AI art environment...");
            // venv preserves the base distribution's managed marker. Re-running repairs an
            // interrupted bootstrap without clearing already installed art packages.
            await RunPython(basePython, "-I -X utf8 -m venv --without-pip \"" + environment + "\"", progress, token);
            progress(0, "Preparing the private image package manager...");
            await RunPython(Python, "-I -X utf8 -m ensurepip --upgrade --default-pip", progress, token);
            await RunPython(Python, "-I -X utf8 -u \"" + Path.Combine(AppFolder(), "setup", "setup_art.py") + "\" \"" + Path.Combine(Data, "models", "sd-turbo") + "\"", progress, token);
            progress(100, "Local AI art ready.");
        }
        internal static async Task RunPython(string executable, string arguments, Action<int, string> progress, CancellationToken token) {
            var start = new ProcessStartInfo(executable, arguments) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true, StandardOutputEncoding = Encoding.UTF8, StandardErrorEncoding = Encoding.UTF8 };
            start.EnvironmentVariables.Remove("HF_HUB_OFFLINE"); start.EnvironmentVariables.Remove("TRANSFORMERS_OFFLINE");
            string failure = null;
            using (var p = Process.Start(start)) using (var tree = new ProcessTree(p)) using (token.Register(() => tree.Stop())) {
                var errors = Task.Run(() => { string line; while ((line = p.StandardError.ReadLine()) != null) WriteLog(line); });
                string s; while ((s = await p.StandardOutput.ReadLineAsync()) != null) { WriteLog(s); if (s.StartsWith("DWA_PROGRESS ")) progress(0, s.Substring(13)); else if (s.StartsWith("DWA_ERROR ")) failure = s.Substring(10); }
                await Task.Run(() => p.WaitForExit()); await errors; token.ThrowIfCancellationRequested();
                if (p.ExitCode != 0) throw new Exception(failure ?? "AI art setup could not finish. View the setup log for the failing step, then choose Retry. Saved campaigns and uploaded artwork are still available.");
            }
        }
        internal static void DeleteOwnedTree(string target, string parent) {
            string full = Path.GetFullPath(target), prefix = Path.GetFullPath(parent).TrimEnd('\\') + "\\";
            if (!full.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) throw new Exception("Refusing to remove a folder outside the app.");
            if (!Directory.Exists(full)) return;
            if ((File.GetAttributes(full) & FileAttributes.ReparsePoint) != 0) throw new Exception("The app folder is a link. Contact the beta organizer for help removing it.");
            foreach (var d in Directory.GetDirectories(full)) DeleteOwnedTree(d, full);
            foreach (var f in Directory.GetFiles(full)) { File.SetAttributes(f, FileAttributes.Normal); File.Delete(f); } Directory.Delete(full);
        }
        internal static int SelfTest() {
            string testParent = Testing ? Root : Path.GetTempPath();
            string temp = Path.Combine(testParent, "dwa-setup-tests-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(temp);
            try {
                foreach (string bad in new[] { "../escape", "a/../../escape", "C:/escape", "a:stream", "a//b", "a/./b", "/absolute" }) {
                    bool rejected = false; try { Child(temp, bad); } catch { rejected = true; } if (!rejected) return 2;
                }
                if (!Child(temp, "folder/ü.txt").StartsWith(temp + "\\")) return 3;
                string file = Path.Combine(temp, "state.json"); AtomicJson(file, new { first = 1 }); AtomicJson(file, new { second = 2 }); if (!File.ReadAllText(file).Contains("second")) return 4;
                string bytes = Path.Combine(temp, "bytes"); File.WriteAllText(bytes, "abc", new UTF8Encoding(false)); if (HashFile(bytes) != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad") return 5;
                bool outside = false; try { DeleteOwnedTree(temp, Path.Combine(temp, "nested")); } catch { outside = true; } if (!outside) return 6;
                if (Testing) {
                    string marker = "concurrent-log-check-" + Guid.NewGuid().ToString("N");
                    Task.WaitAll(Enumerable.Range(0, 40).Select(i => Task.Run(() => WriteLog(marker + "-" + i))).ToArray());
                    if (File.ReadAllLines(Log).Count(line => line.Contains(marker)) != 40) return 7;
                }
                return 0;
            } finally { DeleteOwnedTree(temp, testParent); }
        }
    }
    sealed class SetupForm : Form {
        Label heading, detail, stage, status; CheckBox narrator, art, desktop, open; Button next, cancel, log; ProgressBar progress;
        CancellationTokenSource cancellation; bool busy, installed, finished, configure, downloadFailed;
        public SetupForm(bool configure) {
            this.configure = configure; Text = "Dungeon World AI · Local AI setup"; ClientSize = new Size(640, 540); MinimumSize = Size; MaximumSize = new Size(900, 730); StartPosition = FormStartPosition.CenterScreen; AutoScaleMode = AutoScaleMode.Dpi;
            BackColor = Color.FromArgb(245, 244, 239); ForeColor = Color.FromArgb(33, 44, 36); Font = new Font("Segoe UI", 10); FormBorderStyle = FormBorderStyle.FixedSingle; MaximizeBox = false;
            try { Icon = Icon.ExtractAssociatedIcon(Program.Self); } catch { }
            var banner = new Label { Text = "D  /  DUNGEON WORLD AI", Font = new Font("Segoe UI Semibold", 12), ForeColor = Color.FromArgb(226, 200, 135), BackColor = Color.FromArgb(26, 42, 32), Location = new Point(0, 0), Size = new Size(640, 58), Padding = new Padding(28, 16, 0, 0) }; Controls.Add(banner);
            heading = Label("Welcome to your next adventure", 28, 82, 580, 42); heading.Font = new Font("Segoe UI Semibold", 21);
            detail = Label(configure ? "Add or repair local AI. Once downloaded, your Dungeon Master and artwork generation work offline." : "Add local AI to your table. Downloads are needed once; generation then works offline.", 30, 136, 574, 62);
            narrator = Check("Local AI Dungeon Master · recommended", 30, 218, true);
            Label("One-time download: about 2.5 GB. Works with a CPU; 16 GB RAM recommended.", 53, 248, 555, 42).Font = new Font("Segoe UI", 9);
            art = Check("AI image generation · optional", 30, 302, false);
            Label("Adds several GB. Uses NVIDIA graphics when supported; CPU generation is slower. You can add it later.", 53, 332, 555, 42).Font = new Font("Segoe UI", 9);
            desktop = Check("Create a desktop shortcut", 30, 392, true); desktop.Visible = !configure;
            stage = Label("", 30, 210, 570, 35); stage.Font = new Font("Segoe UI Semibold", 12); stage.Visible = false;
            progress = new ProgressBar { Location = new Point(30, 257), Size = new Size(574, 18), Maximum = 100, Visible = false }; Controls.Add(progress);
            status = Label("", 30, 294, 574, 126); status.Visible = false;
            open = Check("Open Dungeon World AI", 30, 399, true); open.Visible = false;
            log = Button("View setup log", 30, 474, 140); log.Visible = false; log.Click += (s, e) => { if (File.Exists(Program.Log)) Process.Start("notepad.exe", "\"" + Program.Log + "\""); };
            cancel = Button("Cancel", 372, 474, 105); cancel.Click += (s, e) => { if (busy) { cancellation.Cancel(); cancel.Enabled = false; status.Text = "Stopping safely. Partial downloads are kept for Retry."; } else Close(); };
            next = Button(configure ? "Set up AI" : "Install", 490, 474, 114); next.BackColor = Color.FromArgb(44, 88, 61); next.ForeColor = Color.White; next.FlatStyle = FlatStyle.Flat;
            next.Click += async (s, e) => { if (finished) { if (open.Checked) { try { Program.Play(); } catch (Exception ex) { MessageBox.Show(ex.Message); return; } } Close(); } else await Run(); };
            FormClosing += (s, e) => { if (busy) { e.Cancel = true; cancellation.Cancel(); cancel.Enabled = false; status.Text = "Stopping safely. Please wait a moment..."; } };
            AcceptButton = next; CancelButton = cancel;
            Label("BETA · Your saves stay on this device. No subscription or AI account required.", 30, 439, 574, 25).Font = new Font("Segoe UI", 8);
        }
        Label Label(string text, int x, int y, int w, int h) { var c = new Label { Text = text, Location = new Point(x, y), Size = new Size(w, h) }; Controls.Add(c); return c; }
        CheckBox Check(string text, int x, int y, bool check) { var c = new CheckBox { Text = text, Location = new Point(x, y), Size = new Size(574, 30), Checked = check, AutoSize = false }; Controls.Add(c); return c; }
        Button Button(string text, int x, int y, int w) { var c = new Button { Text = text, Location = new Point(x, y), Size = new Size(w, 38), FlatStyle = FlatStyle.System }; Controls.Add(c); return c; }
        void Report(int value, string message) { if (IsDisposed) return; BeginInvoke((Action)(() => { progress.Style = value == 0 ? ProgressBarStyle.Marquee : ProgressBarStyle.Continuous; progress.MarqueeAnimationSpeed = 25; progress.Value = Math.Max(0, Math.Min(100, value)); status.Text = message; })); }
        async Task Run() {
            busy = true; cancellation = new CancellationTokenSource(); next.Enabled = false; cancel.Enabled = true; cancel.Text = "Stop";
            narrator.Visible = art.Visible = desktop.Visible = false;
            foreach (Control c in Controls) if (c is Label && (c.Top == 248 || c.Top == 332)) c.Visible = false;
            stage.Visible = progress.Visible = status.Visible = true; log.Visible = false; heading.Text = "Setting up your table"; detail.Text = "Keep this window open. You can stop safely and retry; your saved campaigns stay on your PC.";
            bool wantNarrator = narrator.Checked, wantArt = art.Checked;
            try {
                Program.ReadInstalled(); installed = true;
                stage.Text = "1 · Check Windows display component"; await Task.Run(() => Program.EnsureWebView(Report, cancellation.Token));
                cancellation.Token.ThrowIfCancellationRequested();
                if (wantNarrator) { stage.Text = "2 · Local Dungeon Master"; await Task.Run(() => Program.Narrator(Report, cancellation.Token)); }
                if (wantArt) { stage.Text = "3 · Local image generation"; await Task.Run(() => Program.Art(Report, cancellation.Token)); }
                heading.Text = "Your table is ready"; detail.Text = "Create a campaign, add your characters, and begin. Use Set up local AI in the app or Start menu whenever you want to add AI features.";
                stage.Text = "Setup complete"; progress.Value = 100; status.Text = "Campaigns save automatically on this device.\n\nYou can remove the app later from Windows Settings. Your adventures and downloaded models will be kept.";
                progress.Style = ProgressBarStyle.Continuous; finished = true; open.Visible = true; next.Text = "Finish"; cancel.Visible = false;
            } catch (OperationCanceledException) {
                heading.Text = installed ? "The app is installed" : "Setup paused"; stage.Text = "Stopped safely"; status.Text = installed ? "You can open the app now or choose Retry to finish local AI. Downloads resume where supported." : "Choose Retry when you are ready. Existing app versions and campaigns are unchanged.";
                downloadFailed = installed;
            } catch (Exception ex) {
                Program.WriteLog(ex.ToString()); heading.Text = installed ? "The app is installed" : "Let’s finish setup"; stage.Text = "One step needs attention"; status.Text = ex.Message; log.Visible = true; downloadFailed = installed;
            } finally {
                busy = false; next.Enabled = true; cancel.Enabled = true; if (!finished) { next.Text = "Retry"; cancel.Text = installed ? "Close" : "Cancel"; }
                if (downloadFailed && !finished) { open.Visible = false; var launch = Controls.Find("launchAfterError", false).FirstOrDefault() as Button; if (launch == null) { launch = Button("Open app now", 190, 474, 140); launch.Name = "launchAfterError"; launch.Click += (s, e) => { try { Program.Play(); Close(); } catch (Exception ex) { MessageBox.Show(ex.Message); } }; } launch.Visible = true; }
                else { var launch = Controls.Find("launchAfterError", false).FirstOrDefault(); if (launch != null) launch.Visible = false; }
            }
        }
    }
    sealed class ProcessTree : IDisposable {
        IntPtr job;
        [StructLayout(LayoutKind.Sequential)] struct Basic { public long a, b; public uint flags; public UIntPtr min, max; public uint active; public UIntPtr affinity; public uint priority, scheduling; }
        [StructLayout(LayoutKind.Sequential)] struct Counters { public ulong a, b, c, d, e, f; }
        [StructLayout(LayoutKind.Sequential)] struct Extended { public Basic basic; public Counters counters; public UIntPtr a, b, c, d; }
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern IntPtr CreateJobObject(IntPtr attributes, string name);
        [DllImport("kernel32.dll")] static extern bool SetInformationJobObject(IntPtr job, int kind, ref Extended info, uint size);
        [DllImport("kernel32.dll")] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
        [DllImport("kernel32.dll")] static extern bool TerminateJobObject(IntPtr job, uint code);
        [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
        internal ProcessTree(Process process) {
            job = CreateJobObject(IntPtr.Zero, null); var info = new Extended(); info.basic.flags = 0x2000;
            if (job == IntPtr.Zero || !SetInformationJobObject(job, 9, ref info, (uint)Marshal.SizeOf(info)) || !AssignProcessToJobObject(job, process.Handle)) { if (job != IntPtr.Zero) CloseHandle(job); job = IntPtr.Zero; try { process.Kill(); } catch { } throw new Exception("Windows could not supervise the image setup process. Close other setup windows and choose Retry."); }
        }
        internal void Stop() { if (job != IntPtr.Zero) TerminateJobObject(job, 1); }
        public void Dispose() { if (job != IntPtr.Zero) { CloseHandle(job); job = IntPtr.Zero; } }
    }
    static class Authenticode {
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] struct FileInfo { public uint size; public string path; public IntPtr handle, subject; }
        [StructLayout(LayoutKind.Sequential)] struct TrustData { public uint size; public IntPtr policy, sip; public uint ui, revocation, choice; public IntPtr file; public uint state; public IntPtr stateData, url; public uint flags, uiContext; }
        [DllImport("wintrust.dll", ExactSpelling = true, CharSet = CharSet.Unicode)] static extern int WinVerifyTrust(IntPtr hwnd, ref Guid action, ref TrustData data);
        internal static bool IsMicrosoft(string path) {
            var info = new FileInfo { size = (uint)Marshal.SizeOf(typeof(FileInfo)), path = path };
            IntPtr memory = Marshal.AllocHGlobal(Marshal.SizeOf(info)); Marshal.StructureToPtr(info, memory, false);
            try {
                var data = new TrustData { size = (uint)Marshal.SizeOf(typeof(TrustData)), ui = 2, choice = 1, file = memory, flags = 0x1000 };
                Guid action = new Guid("00AAC56B-CD44-11d0-8CC2-00C04FC295EE");
                if (WinVerifyTrust(new IntPtr(-1), ref action, ref data) != 0) return false;
                using (var cert = new X509Certificate2(X509Certificate.CreateFromSignedFile(path))) return cert.GetNameInfo(X509NameType.SimpleName, false) == "Microsoft Corporation";
            } catch { return false; } finally { Marshal.DestroyStructure(memory, typeof(FileInfo)); Marshal.FreeHGlobal(memory); }
        }
    }
}

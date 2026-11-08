using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using System.Windows.Forms;
using CefSharp;
using CefSharp.Handler;
using CefSharp.WinForms;

// This browser has no JavaScript object bindings and no connection to campaign storage.
internal sealed class ScryingForm : Form
{
    private ChromiumWebBrowser browser;
    private readonly IntPtr parent;
    private readonly bool smoke;
    private readonly object outputLock = new object();
    private readonly JavaScriptSerializer json = new JavaScriptSerializer();
    private readonly StreamWriter output = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false));
    private string title = "Scrying Pane", address = "about:blank", error = "";
    private bool loading, back, forward;
    private int closing;
    [DllImport("user32.dll")] private static extern IntPtr SetParent(IntPtr child, IntPtr parent);
    [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] private static extern IntPtr GetWindowLongPtr(IntPtr hwnd,int index);
    [DllImport("user32.dll", EntryPoint="SetWindowLongPtrW")] private static extern IntPtr SetWindowLongPtr(IntPtr hwnd,int index,IntPtr value);
    [DllImport("user32.dll")] private static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int width,int height,uint flags);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr hwnd,int command);
    protected override bool ShowWithoutActivation {get{return true;}}
    internal static bool Allowed(string value)
    {
        if(value == "about:blank")return true;
        Uri uri;
        return value != null && value.Length <= 2048 && Uri.TryCreate(value,UriKind.Absolute,out uri) &&
            (uri.Scheme == "https" || uri.Scheme == "http") && String.IsNullOrEmpty(uri.UserInfo) &&
            uri.Host != "tauri.localhost" && !uri.Host.EndsWith(".tauri.localhost",StringComparison.OrdinalIgnoreCase);
    }
    private void Emit(object value){lock(outputLock){output.WriteLine(json.Serialize(value));output.Flush();}}
    private string Clip(string value,int length){value=value??"";return value.Length>length?value.Substring(0,length):value;}
    private void State(){Emit(new {type="state",ready=browser!=null&&browser.IsBrowserInitialized,url=Clip(address,2048),title=Clip(title,120),loading=loading,canGoBack=back,canGoForward=forward,error=Clip(error,1500),cefVersion=Cef.CefVersion,chromiumVersion=Cef.ChromiumVersion});}
    public ScryingForm(IntPtr parentHandle,bool selfTest,int parentPid)
    {
        parent=parentHandle;smoke=selfTest;FormBorderStyle=FormBorderStyle.None;ShowInTaskbar=false;AutoScaleMode=AutoScaleMode.None;
        StartPosition=FormStartPosition.Manual;Bounds=new Rectangle(-32000,-32000,800,500);Text="Dungeon World AI — Scrying Pane";
        browser=new ChromiumWebBrowser("about:blank");browser.Dock=DockStyle.Fill;
        browser.RequestHandler=new NavigationGuard();browser.LifeSpanHandler=new SinglePane(this);
        browser.AddressChanged+=(sender,args)=>{address=args.Address;error="";State();};
        browser.TitleChanged+=(sender,args)=>{title=args.Title;State();};
        browser.LoadingStateChanged+=(sender,args)=>{loading=args.IsLoading;back=args.CanGoBack;forward=args.CanGoForward;State();};
        browser.LoadError+=(sender,args)=>{if(args.ErrorCode!=CefErrorCode.Aborted){error=args.ErrorText;State();}};
        browser.IsBrowserInitializedChanged+=(sender,args)=>{if(browser.IsBrowserInitialized)State();};
        browser.FrameLoadEnd+=(sender,args)=>{if(smoke&&args.Frame.IsMain){Emit(new{type="smoke",ok=true,cefVersion=Cef.CefVersion,chromiumVersion=Cef.ChromiumVersion});Ui(()=>Close());}};
        Controls.Add(browser);
        Shown+=(sender,args)=>{
            if(parent!=IntPtr.Zero){SetParent(Handle,parent);long style=GetWindowLongPtr(Handle,-16).ToInt64();SetWindowLongPtr(Handle,-16,new IntPtr((style|0x40000000L)&~0x80000000L));ShowWindow(Handle,0);}
            if(!smoke){var thread=new Thread(ReadCommands);thread.IsBackground=true;thread.Start();}
        };
        if(parentPid>0){var monitor=new Thread(()=>{try{Process.GetProcessById(parentPid).WaitForExit();}catch{}Ui(()=>Close());});monitor.IsBackground=true;monitor.Start();}
        if(smoke){var timeout=new System.Windows.Forms.Timer();timeout.Interval=20000;timeout.Tick+=(sender,args)=>{Emit(new{type="smoke",ok=false,error="CEF renderer did not load within 20 seconds."});Close();};timeout.Start();}
    }
    internal void Ui(Action action){if(Volatile.Read(ref closing)!=0)return;try{if(IsHandleCreated)BeginInvoke(action);}catch(InvalidOperationException){}}
    private static int Number(Dictionary<string,object> value,string key,int fallback){object raw;return value.TryGetValue(key,out raw)?Convert.ToInt32(raw):fallback;}
    private void ReadCommands()
    {
        try{string line;while((line=Console.ReadLine())!=null){if(line.Length>12000)break;var message=new JavaScriptSerializer().Deserialize<Dictionary<string,object>>(line);Ui(()=>Command(message));}}catch(Exception exception){Emit(new{type="error",error=Clip(exception.Message,1500)});}Ui(()=>Close());
    }
    private void Command(Dictionary<string,object> message)
    {
        try{
            var kind=Convert.ToString(message["kind"]);
            if(kind=="navigate"){string url=Convert.ToString(message["url"]);if(!Allowed(url))throw new ArgumentException("Enter an HTTP or HTTPS address.");error="";browser.Load(url);}
            else if(kind=="back"&&browser.CanGoBack)browser.Back();
            else if(kind=="forward"&&browser.CanGoForward)browser.Forward();
            else if(kind=="reload")browser.Reload();
            else if(kind=="stop")browser.Stop();
            else if(kind=="close")Close();
            else if(kind=="hide")ShowWindow(Handle,0);
            else if(kind=="layout"){
                int x=Number(message,"x",0),y=Number(message,"y",0),width=Number(message,"width",800),height=Number(message,"height",500);
                if(Math.Abs((long)x)>20000||Math.Abs((long)y)>20000||width<1||height<1||width>16000||height>16000)throw new ArgumentException("Invalid pane bounds.");
                SetWindowPos(Handle,IntPtr.Zero,x,y,width,height,0x0010|0x0040);
            }
            State();
        }catch(Exception exception){error=exception.Message;State();}
    }
    protected override void OnFormClosing(FormClosingEventArgs args){Interlocked.Exchange(ref closing,1);base.OnFormClosing(args);}
    protected override void OnFormClosed(FormClosedEventArgs args){browser.Dispose();base.OnFormClosed(args);}
    private sealed class NavigationGuard : RequestHandler
    {
        protected override bool OnBeforeBrowse(IWebBrowser view,IBrowser current,IFrame frame,IRequest request,bool gesture,bool redirect){if(Allowed(request.Url))return false;if(frame!=null&&!frame.IsMain){Uri uri;if(Uri.TryCreate(request.Url,UriKind.Absolute,out uri)&&(uri.Scheme=="about"||uri.Scheme=="blob"||uri.Scheme=="data"||uri.Scheme=="chrome-extension"))return false;}return true;}
    }
    private sealed class SinglePane : LifeSpanHandler
    {
        private readonly ScryingForm form;public SinglePane(ScryingForm owner){form=owner;}
        protected override bool OnBeforePopup(IWebBrowser view,IBrowser current,IFrame frame,string url,string name,WindowOpenDisposition disposition,bool gesture,IPopupFeatures features,IWindowInfo info,IBrowserSettings settings,ref bool noJavascriptAccess,out IWebBrowser newBrowser){newBrowser=null;if(gesture&&Allowed(url))form.Ui(()=>form.browser.Load(url));return true;}
    }
}
internal static class Program
{
    [DllImport("user32.dll")]private static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [STAThread]private static int Main(string[] args)
    {
        try{
            bool smoke=Array.IndexOf(args,"--smoke-test")>=0;string profile=null,log=null;long parent=0;int parentPid=0;
            for(int i=0;i+1<args.Length;i++){if(args[i]=="--profile")profile=args[++i];else if(args[i]=="--log")log=args[++i];else if(args[i]=="--parent")parent=Int64.Parse(args[++i]);else if(args[i]=="--parent-pid")parentPid=Int32.Parse(args[++i]);}
            if(String.IsNullOrEmpty(profile)||!Path.IsPathRooted(profile)||(!smoke&&parent==0))throw new ArgumentException("The desktop must supply a browser profile and parent window.");
            SetProcessDpiAwarenessContext(new IntPtr(-4));Application.EnableVisualStyles();Application.SetCompatibleTextRenderingDefault(false);
            Directory.CreateDirectory(profile);
            var settings=new CefSettings();settings.RootCachePath=profile;settings.CachePath=Path.Combine(profile,"cache");settings.Locale="en-US";settings.LogFile=log??Path.Combine(profile,"cef.log");settings.BrowserSubprocessPath=Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"CefSharp.BrowserSubprocess.exe");
            if(!Cef.Initialize(settings))throw new InvalidOperationException("CEF could not start. Close another Scrying Pane using the same profile and retry.");
            try{Application.Run(new ScryingForm(new IntPtr(parent),smoke,parentPid));}finally{Cef.Shutdown();}return 0;
        }catch(Exception exception){using(var output=new StreamWriter(Console.OpenStandardOutput(),new UTF8Encoding(false))){output.WriteLine(new JavaScriptSerializer().Serialize(new{type="error",error=exception.Message}));}return 1;}
    }
}

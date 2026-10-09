export function SetupPage() {
  return (
    <section className="prose">
      <header className="page-head"><h1>Setup & help</h1></header>
      <div className="card">
        <h2>1. Install Npcap</h2>
        <p>
          The meter reads your own game traffic through Npcap, the same capture driver Wireshark uses. Download it
          from <b>npcap.com</b> and tick <b>"Install Npcap in WinPcap API-compatible Mode"</b> during setup. Then
          press Retry in the yellow banner, or restart the meter.
        </p>
      </div>
      <div className="card">
        <h2>2. Start the meter, then the game</h2>
        <p>
          Starting the meter first lets it see the connection from the beginning. If you start it mid-session it
          still works but may need a zone change before it syncs. If nothing shows up, try running the meter as
          administrator.
        </p>
      </div>
      <div className="card">
        <h2>3. Overlay</h2>
        <p>
          Run the game in <b>borderless windowed</b> mode; exclusive fullscreen hides every overlay. Drag the
          overlay by its top strip (or onto a second monitor), then lock it with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>L</kbd>{" "}
          so clicks go through to the game.
        </p>
      </div>
      <div className="card">
        <h2>What this app does and doesn't do</h2>
        <p>
          It only <b>reads</b> network packets the game already sends to your PC. It never sends, changes or replays
          packets, never reads or writes game memory, and never injects into the game. Third-party tools may still
          be against the game's terms of service, so use it at your own risk.
        </p>
      </div>
    </section>
  );
}

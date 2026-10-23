import AppKit
import Foundation
import SQLite3

func logResult(_ text: String) {
    let output = bookmarkFile().deletingLastPathComponent().appendingPathComponent("probe.log")
    let line = "\(Date()) \(text)\n"
    let bytes = Data(line.utf8)
    if !FileManager.default.fileExists(atPath: output.path) {
        _ = FileManager.default.createFile(atPath: output.path, contents: bytes)
    } else if let handle = try? FileHandle(forWritingTo: output) {
        defer { try? handle.close() }
        do { try handle.seekToEnd(); try handle.write(contentsOf: bytes) } catch { }
    }
    print(text)
}

func check(_ folder: URL) -> Bool {
    let file = folder.appendingPathComponent("History.db").path
    var db: OpaquePointer?
    let status = sqlite3_open_v2(file, &db, SQLITE_OPEN_READONLY, nil)
    guard status == SQLITE_OK, let db else {
        let code = status
        if let db { sqlite3_close(db) }
        logResult("Safari History.db access DENIED/error (SQLite status: \(code)); no URL data read")
        return false
    }
    defer { sqlite3_close(db) }
    var query: OpaquePointer?
    if sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM history_items", -1, &query, nil) == SQLITE_OK {
        defer { sqlite3_finalize(query) }
        if sqlite3_step(query) == SQLITE_ROW {
            let count = sqlite3_column_int(query, 0)
            logResult("Safari History.db read SUCCESS (\(count) local history items; no URLs exported)")
            return true
        }
    }
    logResult("Safari History.db opened, but query did not succeed")
    return false
}

func bookmarkFile() -> URL {
    let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
    let folder = base.appendingPathComponent("RindleScopedProbe", isDirectory: true)
    try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    return folder.appendingPathComponent("safari-bookmark.dat")
}

func verifySavedBookmark() {
    do {
        let bookmark = try Data(contentsOf: bookmarkFile())
        var stale = false
        let folder = try URL(resolvingBookmarkData: bookmark, options: [.withSecurityScope], relativeTo: nil, bookmarkDataIsStale: &stale)
        let allowed = folder.startAccessingSecurityScopedResource()
        defer { if allowed { folder.stopAccessingSecurityScopedResource() } }
        logResult("Bookmark resolved, stale=\(stale), startAccessing=\(allowed)")
        let works = check(folder)
        logResult("PERSISTENCE CHECK: \(works ? "PASS" : "FAIL")")
    } catch {
        logResult("Bookmark resolve FAILED: \(type(of: error)) / \(error.localizedDescription)")
    }
}

if CommandLine.arguments.contains("--verify") {
    verifySavedBookmark()
    exit(0)
}

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let panel = NSOpenPanel()
panel.message = "Select ONLY your Safari history folder. This test grants no Full Disk Access."
panel.prompt = "Allow selected Safari folder"
panel.canChooseDirectories = true
panel.canChooseFiles = false
panel.allowsMultipleSelection = false
panel.canCreateDirectories = false
panel.directoryURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Safari", isDirectory: true)
app.activate(ignoringOtherApps: true)
let outcome = panel.runModal()
if outcome == .OK, let selected = panel.url {
    let expected = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Safari", isDirectory: true).standardizedFileURL
    guard selected.standardizedFileURL == expected else {
        logResult("Rejected selection outside ~/Library/Safari (no broad folder grant stored)")
        app.terminate(nil)
        exit(3)
    }
    logResult("User selected exactly ~/Library/Safari using macOS NSOpenPanel")
    let allowed = selected.startAccessingSecurityScopedResource()
    defer { if allowed { selected.stopAccessingSecurityScopedResource() } }
    logResult("Chooser startAccessing=\(allowed)")
    if check(selected) {
    do {
        let data = try selected.bookmarkData(options: [.withSecurityScope], includingResourceValuesForKeys: nil, relativeTo: nil)
        try data.write(to: bookmarkFile(), options: [.atomic])
        logResult("Saved a security-scoped bookmark in the helper's own sandbox container")
    } catch {
        logResult("Security-scoped bookmark save FAILED: \(error.localizedDescription)")
    }
    } else {
        logResult("History.db remained inaccessible after folder selection; did not save bookmark")
    }
} else {
    logResult("User did not approve the folder picker; no access attempted")
}
app.terminate(nil)

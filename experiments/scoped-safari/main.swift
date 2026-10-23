import AppKit
import Darwin
import Foundation
import SQLite3

if CommandLine.arguments.contains("--serve") {
    runScopedService()
    exit(0)
}

func actualUserHomeDirectory() -> URL? {
    guard let record = getpwuid(getuid()), let cPath = record.pointee.pw_dir else {
        return nil
    }
    // NSHomeDirectory / FileManager.default.homeDirectoryForCurrentUser are
    // remapped to the app's container when running inside App Sandbox.
    return URL(fileURLWithPath: String(cString: cPath), isDirectory: true)
}

func exactSafariFolder() -> URL? {
    actualUserHomeDirectory()?
        .appendingPathComponent("Library/Safari", isDirectory: true)
        .resolvingSymlinksInPath()
        .standardizedFileURL
}

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

/// Confirm the selected folder supports the operations required by Rindle's
/// Safari adapter. Never modify Safari history itself in this experiment.
func checkWriteWithoutTouchingHistory(_ folder: URL) -> Bool {
    let historyPath = folder.appendingPathComponent("History.db").path
    var db: OpaquePointer?
    let result = sqlite3_open_v2(historyPath, &db, SQLITE_OPEN_READWRITE, nil)
    guard result == SQLITE_OK, let db else {
        if let db { sqlite3_close(db) }
        logResult("WRITE CHECK: cannot open existing Safari History.db read/write (SQLite status \(result))")
        return false
    }
    // Opening SQLITE_OPEN_READWRITE does not itself insert or update a visit.
    // Explicitly set query_only to defend against accidental writes by future
    // additions to this proof-of-concept.
    guard sqlite3_exec(db, "PRAGMA query_only=ON", nil, nil, nil) == SQLITE_OK else {
        sqlite3_close(db)
        logResult("WRITE CHECK: SQLite query_only initialization failed")
        return false
    }
    sqlite3_close(db)
    logResult("WRITE CHECK: Safari History.db opened read/write; no SQL mutations performed")

    let probeURL = folder.appendingPathComponent(".rindle-scoped-write-test-\(UUID().uuidString)")
    defer { try? FileManager.default.removeItem(at: probeURL) }
    do {
        let sentinel = Data("Rindle permission test only; no browsing data".utf8)
        try sentinel.write(to: probeURL, options: .withoutOverwriting)
        guard try Data(contentsOf: probeURL) == sentinel else {
            logResult("WRITE CHECK: temporary file verification failed")
            return false
        }
        try FileManager.default.removeItem(at: probeURL)
        logResult("WRITE CHECK: created, verified, and deleted one unrelated test file in selected folder")
        return true
    } catch {
        logResult("WRITE CHECK: file create/read/delete denied (\(error.localizedDescription))")
        return false
    }
}

func bookmarkFile() -> URL {
    let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
    let folder = base.appendingPathComponent("RindleScopedProbe", isDirectory: true)
    try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    return folder.appendingPathComponent("safari-bookmark.dat")
}

func verifySavedBookmark(checkWrites: Bool = false) {
    do {
        let bookmark = try Data(contentsOf: bookmarkFile())
        var stale = false
        let folder = try URL(resolvingBookmarkData: bookmark, options: [.withSecurityScope], relativeTo: nil, bookmarkDataIsStale: &stale)
        let allowed = folder.startAccessingSecurityScopedResource()
        defer { if allowed { folder.stopAccessingSecurityScopedResource() } }
        logResult("Bookmark resolved, stale=\(stale), startAccessing=\(allowed)")
        let works = check(folder)
        logResult("PERSISTENCE CHECK: \(works ? "PASS" : "FAIL")")
        if checkWrites {
            guard let safeFolder = exactSafariFolder(),
                folder.resolvingSymlinksInPath().standardizedFileURL == safeFolder else {
                logResult("WRITE CHECK: selected folder is not ~/Library/Safari; refusing write probe")
                return
            }
            guard works && allowed else {
                logResult("WRITE CHECK: saved permission did not pass the read-only test")
                return
            }
            let writable = checkWriteWithoutTouchingHistory(folder)
            logResult("SCOPED WRITE CHECK: \(writable ? "PASS" : "FAIL")")
        }
    } catch {
        logResult("Bookmark resolve FAILED: \(type(of: error)) / \(error.localizedDescription)")
    }
}

if CommandLine.arguments.contains("--verify") || CommandLine.arguments.contains("--write-check") {
    verifySavedBookmark(checkWrites: CommandLine.arguments.contains("--write-check"))
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
panel.directoryURL = exactSafariFolder()
app.activate(ignoringOtherApps: true)
let outcome = panel.runModal()
if outcome == .OK, let selected = panel.url {
    guard let expected = exactSafariFolder(), selected.resolvingSymlinksInPath().standardizedFileURL == expected else {
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

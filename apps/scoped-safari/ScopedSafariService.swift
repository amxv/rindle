import Foundation
import SQLite3

// Local-only, one-shot JSON-over-stdio. No web server, extension injection,
// remote URLs, arbitrary filesystem paths, or Safari URL navigation.
// Called only after `startAccessingSecurityScopedResource()` succeeds.
struct ScopedRequest: Decodable {
    struct Visit: Decodable {
        let url: String
        let title: String
    }
    let version: Int
    let action: String
    let url: String?
    let limit: Int?
    let visits: [Visit]?
    let dry_run: Bool?
}

enum ScopedError: Error, LocalizedError {
    case denied(String)

    var errorDescription: String? {
        switch self {
        case .denied(let message): return message
        }
    }
}

private let SQLITE_DYNAMIC = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

final class ScopedDatabase {
    let handle: OpaquePointer

    init(path: String, writable: Bool) throws {
        var connection: OpaquePointer?
        let flags = writable ? SQLITE_OPEN_READWRITE : SQLITE_OPEN_READONLY
        let code = sqlite3_open_v2(path, &connection, flags, nil)
        guard code == SQLITE_OK, let connection else {
            if let connection { sqlite3_close(connection) }
            throw ScopedError.denied("Safari history database cannot be opened (SQLite \(code))")
        }
        handle = connection
        sqlite3_busy_timeout(handle, 8000)
    }

    deinit { sqlite3_close(handle) }

    func statement<T>(_ sql: String, _ f: (OpaquePointer) throws -> T) throws -> T {
        var query: OpaquePointer?
        guard sqlite3_prepare_v2(handle, sql, -1, &query, nil) == SQLITE_OK, let query else {
            if let query { sqlite3_finalize(query) }
            throw ScopedError.denied("Safari history schema incompatible with Rindle")
        }
        defer { sqlite3_finalize(query) }
        return try f(query)
    }

    func int(_ sql: String) throws -> Int64 {
        try statement(sql) { query in
            guard sqlite3_step(query) == SQLITE_ROW else {
                throw ScopedError.denied("Safari history query did not return an integer")
            }
            return sqlite3_column_int64(query, 0)
        }
    }

    func execute(_ sql: String) throws {
        guard sqlite3_exec(handle, sql, nil, nil, nil) == SQLITE_OK else {
            throw ScopedError.denied("Safari history database transaction failed")
        }
    }

    func bind(_ value: String, to query: OpaquePointer, at index: Int32) {
        sqlite3_bind_text(query, index, value, -1, SQLITE_DYNAMIC)
    }

    func visitGeneration(_ url: String) throws -> Int64? {
        try statement("SELECT MAX(v.generation) FROM history_visits v JOIN history_items i ON i.id=v.history_item WHERE i.url=?1") { query in
            bind(url, to: query, at: 1)
            guard sqlite3_step(query) == SQLITE_ROW else {
                throw ScopedError.denied("Unable to read Safari visit generation")
            }
            return sqlite3_column_type(query, 0) == SQLITE_NULL ? nil : sqlite3_column_int64(query, 0)
        }
    }

    func contains(_ url: String) throws -> Bool {
        try statement("SELECT 1 FROM history_items WHERE url=?1 LIMIT 1") { query in
            bind(url, to: query, at: 1)
            let step = sqlite3_step(query)
            if step == SQLITE_ROW { return true }
            if step == SQLITE_DONE { return false }
            throw ScopedError.denied("Safari history URL lookup failed")
        }
    }

    func remoteURLs(max count: Int) throws -> [String] {
        try statement("SELECT i.url FROM history_items i JOIN history_visits v ON v.history_item=i.id WHERE v.origin=1 GROUP BY i.id ORDER BY MAX(v.visit_time) DESC LIMIT ?1") { query in
            sqlite3_bind_int(query, 1, Int32(count))
            var urls: [String] = []
            var step = sqlite3_step(query)
            while step == SQLITE_ROW {
                if let ptr = sqlite3_column_text(query, 0) {
                    urls.append(String(cString: UnsafeRawPointer(ptr).assumingMemoryBound(to: CChar.self)))
                }
                step = sqlite3_step(query)
            }
            guard step == SQLITE_DONE else {
                throw ScopedError.denied("Safari iCloud history query did not complete")
            }
            return urls
        }
    }

    func validateForWrites() throws {
        let result = try statement("PRAGMA quick_check") { query in
            guard sqlite3_step(query) == SQLITE_ROW, let ptr = sqlite3_column_text(query, 0) else {
                return "failed"
            }
            return String(cString: UnsafeRawPointer(ptr).assumingMemoryBound(to: CChar.self))
        }
        guard result == "ok" else {
            throw ScopedError.denied("Safari history integrity check failed; refusing to write")
        }
        for (table, expected) in [
            ("history_items", ["id", "url", "domain_expansion", "visit_count", "daily_visit_counts", "visit_count_score", "status_code"]),
            ("history_visits", ["history_item", "visit_time", "title", "origin", "generation", "attributes", "score"]),
            ("metadata", ["key", "value"]),
        ] {
            let names: Set<String> = try statement("PRAGMA table_info(\(table))") { query in
                var cols: Set<String> = []
                while sqlite3_step(query) == SQLITE_ROW {
                    if let ptr = sqlite3_column_text(query, 1) {
                        cols.insert(String(cString: UnsafeRawPointer(ptr).assumingMemoryBound(to: CChar.self)))
                    }
                }
                return cols
            }
            guard Set(expected).isSubset(of: names) else {
                throw ScopedError.denied("Safari history schema has changed; refusing to write")
            }
        }
    }

    func backup() throws {
        let directory = bookmarkFile().deletingLastPathComponent().appendingPathComponent("backups", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let name = "safari-\(UInt64(Date().timeIntervalSince1970 * 1_000_000))-\(ProcessInfo.processInfo.processIdentifier).sqlite"
        let path = directory.appendingPathComponent(name)
        guard !FileManager.default.fileExists(atPath: path.path) else {
            throw ScopedError.denied("Refusing to overwrite a Safari backup")
        }
        try statement("VACUUM INTO ?1") { query in
            bind(path.path, to: query, at: 1)
            guard sqlite3_step(query) == SQLITE_DONE else {
                throw ScopedError.denied("Unable to make a consistent Safari database backup")
            }
        }
        try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: path.path)
        let backups = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
            .filter { $0.lastPathComponent.hasPrefix("safari-") && $0.pathExtension == "sqlite" }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
        if backups.count > 3 {
            for old in backups.dropLast(3) { try FileManager.default.removeItem(at: old) }
        }
    }

    func insert(_ visits: [ScopedRequest.Visit], dryRun: Bool) throws -> [[String: Any]] {
        guard visits.count <= 1000 else { throw ScopedError.denied("Safari insert batch exceeds 1000 URLs") }
        if visits.isEmpty { return [] }
        try validateForWrites()
        let missing = try visits.filter { try !contains($0.url) }
        if missing.isEmpty { return visits.map { ["url": $0.url, "generation": NSNull()] } }
        if !dryRun { try backup() }
        let next = try int("SELECT CAST(value AS INTEGER) FROM metadata WHERE key='current_generation'") + 1
        let now = Date().timeIntervalSince1970 - 978307200.0
        try execute("BEGIN IMMEDIATE")
        var committed = false
        defer { if !committed { try? execute("ROLLBACK") } }

        var output: [[String: Any]] = []
        for item in visits {
            if try contains(item.url) {
                output.append(["url": item.url, "generation": NSNull()])
                continue
            }
            guard let expansion = safariDomainExpansion(item.url) else {
                throw ScopedError.denied("Refusing to insert invalid Safari URL")
            }
            try statement("INSERT INTO history_items(url,domain_expansion,visit_count,daily_visit_counts,weekly_visit_counts,autocomplete_triggers,should_recompute_derived_visit_counts,visit_count_score,status_code) VALUES (?1,?2,1,?3,NULL,NULL,0,100,0)") { query in
                bind(item.url, to: query, at: 1)
                bind(expansion, to: query, at: 2)
                let visitCounts: [UInt8] = [100, 0, 0, 0]
                let status = visitCounts.withUnsafeBytes { bytes in
                    sqlite3_bind_blob(query, 3, bytes.baseAddress, Int32(visitCounts.count), SQLITE_DYNAMIC)
                }
                guard status == SQLITE_OK, sqlite3_step(query) == SQLITE_DONE else {
                    throw ScopedError.denied("Safari history URL insert failed")
                }
            }
            let itemID = sqlite3_last_insert_rowid(handle)
            try statement("INSERT INTO history_visits(history_item,visit_time,title,load_successful,http_non_get,synthesized,origin,generation,attributes,score) VALUES (?1,?2,?3,1,0,0,0,?4,0,100)") { query in
                sqlite3_bind_int64(query, 1, itemID)
                sqlite3_bind_double(query, 2, now)
                bind(String(item.title.prefix(512)), to: query, at: 3)
                sqlite3_bind_int64(query, 4, next)
                guard sqlite3_step(query) == SQLITE_DONE else {
                    throw ScopedError.denied("Safari history visit insert failed")
                }
            }
            output.append(["url": item.url, "generation": next])
        }
        if dryRun {
            // Exercise the actual private Safari database write permissions,
            // constraints and transaction logic without leaving test visits.
            try execute("ROLLBACK")
            committed = true
            return output
        }
        try execute("COMMIT")
        committed = true
        return output
    }
}

func safariDomainExpansion(_ raw: String) -> String? {
    guard let url = URLComponents(string: raw),
          let scheme = url.scheme?.lowercased(),
          scheme == "https" || scheme == "http",
          url.user == nil, url.password == nil,
          let host = url.host?.lowercased(), host.contains("."), raw.count <= 8192 else { return nil }
    if host == "localhost" || host.hasSuffix(".local") || host.hasSuffix(".internal") { return nil }
    let components = host.split(separator: ".").map(String.init)
    if components.count < 2 { return nil }
    let suffix = components.suffix(2).joined(separator: ".")
    let multiPart = Set(["co.uk", "org.uk", "ac.uk", "gov.uk", "com.au", "com.br", "co.in", "co.jp", "co.nz"])
    return components.dropLast(multiPart.contains(suffix) ? 2 : 1).joined(separator: ".")
}

func runScopedService() {
    do {
        let bytes = FileHandle.standardInput.readDataToEndOfFile()
        guard !bytes.isEmpty && bytes.count <= 4_000_000 else {
            throw ScopedError.denied("Invalid local Safari helper request size")
        }
        let req = try JSONDecoder().decode(ScopedRequest.self, from: bytes)
        guard req.version == 1 else { throw ScopedError.denied("Unsupported Safari helper protocol version") }
        let bookmarkData = try Data(contentsOf: bookmarkFile())
        var stale = false
        let folder = try URL(resolvingBookmarkData: bookmarkData, options: [.withSecurityScope], relativeTo: nil, bookmarkDataIsStale: &stale)
        guard !stale, let expected = exactSafariFolder(),
            folder.resolvingSymlinksInPath().standardizedFileURL == expected else {
            throw ScopedError.denied("Safari folder bookmark is stale or points outside ~/Library/Safari")
        }
        guard folder.startAccessingSecurityScopedResource() else {
            throw ScopedError.denied("Safari folder-specific access was denied; reselect it with the helper")
        }
        defer { folder.stopAccessingSecurityScopedResource() }
        let writer = req.action == "insert"
        let db = try ScopedDatabase(path: folder.appendingPathComponent("History.db").path, writable: writer)
        var response: [String: Any] = ["ok": true]
        switch req.action {
        case "inspect":
            let maxCount = max(1, min(req.limit ?? 10000, 20000))
            response["current_generation"] = try db.int("SELECT CAST(value AS INTEGER) FROM metadata WHERE key='current_generation'")
            response["last_synced_generation"] = try db.int("SELECT CAST(value AS INTEGER) FROM metadata WHERE key='last_synced_generation'")
            response["remote_urls"] = try db.remoteURLs(max: maxCount)
        case "checkpoint":
            response["current_generation"] = try db.int("SELECT CAST(value AS INTEGER) FROM metadata WHERE key='current_generation'")
            response["last_synced_generation"] = try db.int("SELECT CAST(value AS INTEGER) FROM metadata WHERE key='last_synced_generation'")
        case "visit_generation":
            guard let url = req.url, url.count <= 8192 else { throw ScopedError.denied("Missing URL for generation check") }
            response["generation"] = try db.visitGeneration(url) as Any? ?? NSNull()
        case "insert":
            guard let visits = req.visits else { throw ScopedError.denied("Missing Safari insert batch") }
            let dryRun = req.dry_run == true
            response["imports"] = try db.insert(visits, dryRun: dryRun)
            response["rolled_back"] = dryRun
        default:
            throw ScopedError.denied("Unsupported Safari helper operation")
        }
        let data = try JSONSerialization.data(withJSONObject: response, options: [.fragmentsAllowed])
        FileHandle.standardOutput.write(data)
        FileHandle.standardOutput.write(Data([10]))
    } catch {
        // Do not leak history URLs, selected paths or database contents into
        // LaunchAgent logs. Untrusted caller input is never interpolated here.
        let message = error.localizedDescription
        fputs("Rindle scoped Safari helper: \(message)\n", stderr)
        exit(1)
    }
}

const std = @import("std");
const c = @cImport(@cInclude("sqlite3.h"));
const preview = @import("preview.zig");

const global_io = std.Options.debug_io;

pub const Error = error{ Open, Exec, Prepare, Bind, Step, NotFound };

const SQLITE_STATIC: c.sqlite3_destructor_type = null;
const SQLITE_TRANSIENT: c.sqlite3_destructor_type = @ptrFromInt(@as(usize, @bitCast(@as(isize, -1))));

/// Layout generation stamped into `PRAGMA user_version`. v1 kept the payload in
/// `items.data`; v2 moves it to `blobs` and stores a rendered preview, so `list`
/// never has to touch a payload page.
const schema_version = 2;

const Digest = [std.crypto.hash.sha2.Sha256.digest_length]u8;

fn ensureDir(path: []const u8) void {
    if (std.fs.path.dirname(path)) |parent| {
        std.Io.Dir.createDirPath(.cwd(), global_io, parent) catch {};
    }
}

fn hashOf(data: []const u8) Digest {
    var digest: Digest = undefined;
    std.crypto.hash.sha2.Sha256.hash(data, &digest, .{});
    return digest;
}

pub const Db = struct {
    handle: *c.sqlite3,
    /// Previews are rendered on store, so the width in force then is the width
    /// baked into the row. `reindex` re-renders the history under a new one.
    preview_width: u64,

    pub fn open(allocator: std.mem.Allocator, path: []const u8, preview_width: u64) !Db {
        ensureDir(path);

        const path_z = try allocator.dupeZ(u8, path);
        defer allocator.free(path_z);

        var handle: ?*c.sqlite3 = null;
        const rc = c.sqlite3_open_v2(
            path_z,
            &handle,
            c.SQLITE_OPEN_READWRITE | c.SQLITE_OPEN_CREATE,
            null,
        );
        if (rc != c.SQLITE_OK or handle == null) return Error.Open;

        const db = Db{ .handle = handle.?, .preview_width = preview_width };
        try db.initSchema(allocator);
        return db;
    }

    pub fn close(self: *const Db) void {
        _ = c.sqlite3_close(self.handle);
    }

    fn exec(self: *const Db, sql: [*:0]const u8) !void {
        var errmsg: [*c]u8 = null;
        const rc = c.sqlite3_exec(self.handle, sql, null, null, &errmsg);
        if (errmsg != null) c.sqlite3_free(errmsg);
        if (rc != c.SQLITE_OK) return Error.Exec;
    }

    fn prepare(self: *const Db, sql: [*:0]const u8) !?*c.sqlite3_stmt {
        var stmt: ?*c.sqlite3_stmt = null;
        if (c.sqlite3_prepare_v2(self.handle, sql, -1, &stmt, null) != c.SQLITE_OK) {
            _ = c.sqlite3_finalize(stmt);
            return Error.Prepare;
        }
        return stmt;
    }

    fn initSchema(self: *const Db, allocator: std.mem.Allocator) !void {
        try self.exec("PRAGMA journal_mode=WAL");
        try self.exec("PRAGMA synchronous=NORMAL");
        // blobs are removed with their item; every DELETE below targets `items`.
        try self.exec("PRAGMA foreign_keys=ON");
        // `cliphizt list | cliphizt delete` has two processes on the db at once,
        // and `list` opens with a write (expiry purge); wait instead of failing.
        try self.exec("PRAGMA busy_timeout=2000");

        // An up-to-date db needs no DDL, and skipping it keeps every invocation
        // read-only up to the command's own work.
        if (try self.userVersion() == schema_version) return;

        if (self.hasLegacyPayloadColumn()) try self.migrateFromV1(allocator);

        try self.exec(
            \\CREATE TABLE IF NOT EXISTS items (
            \\    id         INTEGER PRIMARY KEY AUTOINCREMENT,
            \\    created_at INTEGER NOT NULL,
            \\    expires_at INTEGER,
            \\    size       INTEGER NOT NULL,
            \\    hash       BLOB    NOT NULL,
            \\    preview    TEXT    NOT NULL
            \\);
            \\CREATE TABLE IF NOT EXISTS blobs (
            \\    item_id INTEGER PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
            \\    data    BLOB NOT NULL
            \\);
            \\CREATE INDEX IF NOT EXISTS idx_created_at ON items(created_at DESC, id DESC);
            \\CREATE INDEX IF NOT EXISTS idx_hash ON items(hash);
            \\CREATE INDEX IF NOT EXISTS idx_expires_at ON items(expires_at)
            \\    WHERE expires_at IS NOT NULL;
        );
        try self.exec(std.fmt.comptimePrint("PRAGMA user_version={d}", .{schema_version}));
    }

    fn userVersion(self: *const Db) !i64 {
        const stmt = try self.prepare("PRAGMA user_version");
        defer _ = c.sqlite3_finalize(stmt);
        if (c.sqlite3_step(stmt) != c.SQLITE_ROW) return Error.Step;
        return c.sqlite3_column_int64(stmt, 0);
    }

    /// A v1 `items` carries the payload inline; asking sqlite to compile a
    /// reference to that column is the version-proof way to detect it.
    fn hasLegacyPayloadColumn(self: *const Db) bool {
        var stmt: ?*c.sqlite3_stmt = null;
        const rc = c.sqlite3_prepare_v2(self.handle, "SELECT data FROM items LIMIT 0", -1, &stmt, null);
        _ = c.sqlite3_finalize(stmt);
        return rc == c.SQLITE_OK;
    }

    /// Splits a v1 database in place: metadata (with a rendered preview and a
    /// content hash) into the new `items`, payloads into `blobs`. Single
    /// transaction, so an interrupted upgrade leaves the old layout intact.
    fn migrateFromV1(self: *const Db, allocator: std.mem.Allocator) !void {
        try self.exec("BEGIN IMMEDIATE");
        errdefer self.exec("ROLLBACK") catch {};

        try self.exec("ALTER TABLE items RENAME TO items_v1");
        // Indexes survive the rename under their old names and would collide.
        try self.exec("DROP INDEX IF EXISTS idx_created_at");
        try self.exec("DROP INDEX IF EXISTS idx_expires_at");

        try self.exec(
            \\CREATE TABLE items (
            \\    id         INTEGER PRIMARY KEY AUTOINCREMENT,
            \\    created_at INTEGER NOT NULL,
            \\    expires_at INTEGER,
            \\    size       INTEGER NOT NULL,
            \\    hash       BLOB    NOT NULL,
            \\    preview    TEXT    NOT NULL
            \\);
            \\CREATE TABLE IF NOT EXISTS blobs (
            \\    item_id INTEGER PRIMARY KEY REFERENCES items(id) ON DELETE CASCADE,
            \\    data    BLOB NOT NULL
            \\);
        );

        const read = try self.prepare("SELECT id, data, created_at, expires_at FROM items_v1 ORDER BY id");
        defer _ = c.sqlite3_finalize(read);

        const write = try self.prepare(
            "INSERT INTO items (id, created_at, expires_at, size, hash, preview) VALUES (?, ?, ?, ?, ?, ?)",
        );
        defer _ = c.sqlite3_finalize(write);

        while (c.sqlite3_step(read) == c.SQLITE_ROW) {
            const id = c.sqlite3_column_int64(read, 0);
            const blob_ptr = c.sqlite3_column_blob(read, 1);
            const blob_len: usize = @intCast(c.sqlite3_column_bytes(read, 1));
            if (blob_ptr == null) continue;
            const data: []const u8 = @as([*]const u8, @ptrCast(blob_ptr))[0..blob_len];

            const text = try preview.render(allocator, data, self.preview_width);
            defer allocator.free(text);
            const digest = hashOf(data);

            _ = c.sqlite3_reset(write);
            if (c.sqlite3_bind_int64(write, 1, id) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_bind_int64(write, 2, c.sqlite3_column_int64(read, 2)) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_column_type(read, 3) == c.SQLITE_NULL) {
                if (c.sqlite3_bind_null(write, 3) != c.SQLITE_OK) return Error.Bind;
            } else {
                if (c.sqlite3_bind_int64(write, 3, c.sqlite3_column_int64(read, 3)) != c.SQLITE_OK) return Error.Bind;
            }
            if (c.sqlite3_bind_int64(write, 4, @intCast(blob_len)) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_bind_blob(write, 5, &digest, digest.len, SQLITE_TRANSIENT) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_bind_text(write, 6, text.ptr, @intCast(text.len), SQLITE_TRANSIENT) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_step(write) != c.SQLITE_DONE) return Error.Step;
        }

        try self.exec("INSERT INTO blobs (item_id, data) SELECT id, data FROM items_v1");
        try self.exec("DROP TABLE items_v1");
        try self.exec("COMMIT");

        // The payload pages the old table held are free now but the file keeps
        // them; reclaim once, here, rather than leaving a third of it as slack.
        try self.exec("VACUUM");
    }

    pub fn purgeExpired(self: *const Db) !void {
        try self.exec("DELETE FROM items WHERE expires_at IS NOT NULL AND expires_at < unixepoch()");
    }

    pub fn store(
        self: *const Db,
        allocator: std.mem.Allocator,
        data: []const u8,
        expires_at: ?i64,
        max_items: u64,
        max_dedupe: u64,
    ) !i64 {
        try self.purgeExpired();

        const digest = hashOf(data);
        const text = try preview.render(allocator, data, self.preview_width);
        defer allocator.free(text);

        try self.exec("BEGIN IMMEDIATE");
        errdefer self.exec("ROLLBACK") catch {};

        try self.deleteDupe(&digest, data.len, max_dedupe);

        const meta = try self.prepare(
            "INSERT INTO items (created_at, expires_at, size, hash, preview) VALUES (unixepoch(), ?, ?, ?, ?)",
        );
        defer _ = c.sqlite3_finalize(meta);

        if (expires_at) |exp| {
            if (c.sqlite3_bind_int64(meta, 1, exp) != c.SQLITE_OK) return Error.Bind;
        } else {
            if (c.sqlite3_bind_null(meta, 1) != c.SQLITE_OK) return Error.Bind;
        }
        if (c.sqlite3_bind_int64(meta, 2, @intCast(data.len)) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_bind_blob(meta, 3, &digest, digest.len, SQLITE_STATIC) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_bind_text(meta, 4, text.ptr, @intCast(text.len), SQLITE_STATIC) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(meta) != c.SQLITE_DONE) return Error.Step;

        const id = c.sqlite3_last_insert_rowid(self.handle);

        const payload = try self.prepare("INSERT INTO blobs (item_id, data) VALUES (?, ?)");
        defer _ = c.sqlite3_finalize(payload);

        if (c.sqlite3_bind_int64(payload, 1, id) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_bind_blob(payload, 2, data.ptr, @intCast(data.len), SQLITE_STATIC) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(payload) != c.SQLITE_DONE) return Error.Step;

        try self.trimToMax(max_items);
        try self.exec("COMMIT");
        return id;
    }

    /// Drops an earlier copy of the same payload within the `max_dedupe` most
    /// recent entries. Matching on (hash, size) keeps this index-only: no
    /// payload page is read, which is what a per-copy code path can afford.
    fn deleteDupe(self: *const Db, digest: *const Digest, size: usize, max_dedupe: u64) !void {
        const stmt = try self.prepare(
            \\DELETE FROM items WHERE id IN (
            \\    SELECT id FROM (
            \\        SELECT id, hash, size FROM items ORDER BY created_at DESC, id DESC LIMIT ?
            \\    ) WHERE hash = ? AND size = ?
            \\)
        );
        defer _ = c.sqlite3_finalize(stmt);

        if (c.sqlite3_bind_int64(stmt, 1, @intCast(max_dedupe)) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_bind_blob(stmt, 2, digest, digest.len, SQLITE_STATIC) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_bind_int64(stmt, 3, @intCast(size)) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(stmt) != c.SQLITE_DONE) return Error.Step;
    }

    fn trimToMax(self: *const Db, max_items: u64) !void {
        const stmt = try self.prepare(
            "DELETE FROM items WHERE id IN (SELECT id FROM items ORDER BY created_at DESC LIMIT -1 OFFSET ?)",
        );
        defer _ = c.sqlite3_finalize(stmt);

        if (c.sqlite3_bind_int64(stmt, 1, @intCast(max_items)) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(stmt) != c.SQLITE_DONE) return Error.Step;
    }

    /// Reads previews only: the whole listing is a covering scan of `items`,
    /// tens of KiB regardless of how many megabytes of payload sit in `blobs`.
    pub fn list(self: *const Db) !void {
        try self.purgeExpired();

        var write_buf: [65536]u8 = undefined;
        var writer = std.Io.File.writer(.stdout(), global_io, &write_buf);

        const stmt = try self.prepare("SELECT id, preview FROM items ORDER BY created_at DESC, id DESC");
        defer _ = c.sqlite3_finalize(stmt);

        while (c.sqlite3_step(stmt) == c.SQLITE_ROW) {
            const id = c.sqlite3_column_int64(stmt, 0);
            const text_ptr = c.sqlite3_column_text(stmt, 1);
            const text_len: usize = @intCast(c.sqlite3_column_bytes(stmt, 1));
            const text: []const u8 = if (text_ptr == null)
                ""
            else
                @as([*]const u8, @ptrCast(text_ptr))[0..text_len];

            try writer.interface.print("{d}\t{s}\n", .{ id, text });
        }

        try writer.interface.flush();
    }

    /// Re-renders every preview at the current width. Unlike `list` this walks
    /// the payloads, so it is a maintenance command, not a hot path.
    pub fn reindex(self: *const Db, allocator: std.mem.Allocator) !u64 {
        try self.exec("BEGIN IMMEDIATE");
        errdefer self.exec("ROLLBACK") catch {};

        const read = try self.prepare("SELECT item_id, data FROM blobs");
        defer _ = c.sqlite3_finalize(read);

        const write = try self.prepare("UPDATE items SET preview = ?, size = ?, hash = ? WHERE id = ?");
        defer _ = c.sqlite3_finalize(write);

        var count: u64 = 0;
        while (c.sqlite3_step(read) == c.SQLITE_ROW) {
            const id = c.sqlite3_column_int64(read, 0);
            const blob_ptr = c.sqlite3_column_blob(read, 1);
            const blob_len: usize = @intCast(c.sqlite3_column_bytes(read, 1));
            if (blob_ptr == null) continue;
            const data: []const u8 = @as([*]const u8, @ptrCast(blob_ptr))[0..blob_len];

            const text = try preview.render(allocator, data, self.preview_width);
            defer allocator.free(text);
            const digest = hashOf(data);

            _ = c.sqlite3_reset(write);
            if (c.sqlite3_bind_text(write, 1, text.ptr, @intCast(text.len), SQLITE_TRANSIENT) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_bind_int64(write, 2, @intCast(blob_len)) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_bind_blob(write, 3, &digest, digest.len, SQLITE_TRANSIENT) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_bind_int64(write, 4, id) != c.SQLITE_OK) return Error.Bind;
            if (c.sqlite3_step(write) != c.SQLITE_DONE) return Error.Step;
            count += 1;
        }

        try self.exec("COMMIT");
        return count;
    }

    pub fn decodeToStdout(self: *const Db, allocator: std.mem.Allocator, id: i64) !void {
        const stmt = try self.prepare("SELECT data FROM blobs WHERE item_id = ?");
        defer _ = c.sqlite3_finalize(stmt);

        if (c.sqlite3_bind_int64(stmt, 1, id) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(stmt) != c.SQLITE_ROW) return Error.NotFound;

        const blob_ptr = c.sqlite3_column_blob(stmt, 0);
        const blob_len: usize = @intCast(c.sqlite3_column_bytes(stmt, 0));
        if (blob_ptr == null) return Error.NotFound;

        const data: []const u8 = @as([*]const u8, @ptrCast(blob_ptr))[0..blob_len];
        const copy = try allocator.dupe(u8, data);
        defer allocator.free(copy);

        try std.Io.File.writeStreamingAll(.stdout(), global_io, copy);
    }

    pub fn deleteById(self: *const Db, allocator: std.mem.Allocator, id: i64) !void {
        _ = allocator;
        const stmt = try self.prepare("DELETE FROM items WHERE id = ?");
        defer _ = c.sqlite3_finalize(stmt);

        if (c.sqlite3_bind_int64(stmt, 1, id) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(stmt) != c.SQLITE_DONE) return Error.Step;
    }

    pub fn deleteByQuery(self: *const Db, query: []const u8) !void {
        const stmt = try self.prepare(
            "DELETE FROM items WHERE id IN (SELECT item_id FROM blobs WHERE instr(data, ?) > 0)",
        );
        defer _ = c.sqlite3_finalize(stmt);

        if (c.sqlite3_bind_blob(stmt, 1, query.ptr, @intCast(query.len), SQLITE_STATIC) != c.SQLITE_OK) return Error.Bind;
        if (c.sqlite3_step(stmt) != c.SQLITE_DONE) return Error.Step;
    }

    pub fn wipe(self: *const Db) !void {
        try self.exec("DELETE FROM items");
        try self.exec("DELETE FROM blobs");
        try self.exec("VACUUM");
    }

    pub fn compact(self: *const Db) !void {
        try self.exec("VACUUM");
    }

    pub fn cleanup(self: *const Db) !void {
        try self.purgeExpired();
    }

    pub fn deleteMostRecent(self: *const Db) !void {
        try self.exec(
            "DELETE FROM items WHERE id = (SELECT id FROM items ORDER BY created_at DESC, id DESC LIMIT 1)",
        );
    }
};

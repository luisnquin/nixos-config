const std = @import("std");
const Db = @import("db.zig").Db;
const Config = @import("config.zig").Config;

const global_io = std.Options.debug_io;

pub fn runDelete(allocator: std.mem.Allocator, cfg: *const Config) !void {
    var stdin_buf: [65536]u8 = undefined;
    var reader = std.Io.File.reader(.stdin(), global_io, &stdin_buf);
    const input = try reader.interface.allocRemaining(allocator, std.Io.Limit.limited64(64 * 1024 * 1024));
    defer allocator.free(input);

    const db = try Db.open(allocator, cfg.db_path, cfg.preview_width);
    defer db.close();

    var lines = std.mem.splitScalar(u8, input, '\n');
    while (lines.next()) |raw| {
        const line = std.mem.trim(u8, raw, "\r");
        if (line.len == 0) continue;
        const id = extractId(line) orelse continue;
        db.deleteById(allocator, id) catch {};
    }
}

pub fn runDeleteQuery(allocator: std.mem.Allocator, cfg: *const Config, query: []const u8) !void {
    const db = try Db.open(allocator, cfg.db_path, cfg.preview_width);
    defer db.close();
    try db.deleteByQuery(query);
}

pub fn runWipe(allocator: std.mem.Allocator, cfg: *const Config) !void {
    const db = try Db.open(allocator, cfg.db_path, cfg.preview_width);
    defer db.close();
    try db.wipe();
}

fn extractId(line: []const u8) ?i64 {
    const tab_pos = std.mem.indexOfScalar(u8, line, '\t') orelse {
        return std.fmt.parseInt(i64, line, 10) catch null;
    };
    return std.fmt.parseInt(i64, line[0..tab_pos], 10) catch null;
}

const std = @import("std");
const Db = @import("db.zig").Db;
const Config = @import("config.zig").Config;
const ttl = @import("ttl.zig");

const global_io = std.Options.debug_io;

pub fn run(allocator: std.mem.Allocator, cfg: *const Config, args: []const []const u8) !void {
    const state: [:0]const u8 = if (std.c.getenv("CLIPBOARD_STATE")) |p| std.mem.span(p) else "";

    if (std.mem.eql(u8, state, "sensitive")) return;

    const db = try Db.open(allocator, cfg.db_path, cfg.preview_width);
    defer db.close();

    if (std.mem.eql(u8, state, "clear")) {
        try db.deleteMostRecent();
        return;
    }

    var expires_at: ?i64 = null;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--ttl") or std.mem.eql(u8, args[i], "-ttl")) {
            i += 1;
            if (i >= args.len) {
                std.debug.print("error: --ttl requires a value\n", .{});
                std.process.exit(1);
            }
            const dur = ttl.parse(args[i]) catch {
                std.debug.print("error: invalid TTL value: {s}\n", .{args[i]});
                std.process.exit(1);
            };
            expires_at = unixTimestamp() + @as(i64, @intCast(dur));
        }
    }

    if (expires_at == null) {
        const mode = try readMode(allocator, cfg);
        defer allocator.free(mode);
        if (std.mem.eql(u8, mode, "ephemeral")) {
            expires_at = unixTimestamp() + @as(i64, @intCast(cfg.ephemeral_ttl));
        }
    }

    var stdin_buf: [65536]u8 = undefined;
    var reader = std.Io.File.reader(.stdin(), global_io, &stdin_buf);
    const data = reader.interface.allocRemaining(allocator, std.Io.Limit.limited64(cfg.max_store_size + 1)) catch |e| switch (e) {
        error.StreamTooLong => return,
        else => return e,
    };
    defer allocator.free(data);

    if (data.len > cfg.max_store_size) return;

    const trimmed = std.mem.trim(u8, data, " \t\n\r");
    if (trimmed.len == 0) return;

    if (cfg.min_store_length > 0) {
        if (countCodepoints(data) < cfg.min_store_length) return;
    }

    _ = try db.store(allocator, data, expires_at, cfg.max_items, cfg.max_dedupe_search);
}

fn countCodepoints(data: []const u8) u64 {
    var count: u64 = 0;
    var pos: usize = 0;
    while (pos < data.len) {
        const clen: usize = std.unicode.utf8ByteSequenceLength(data[pos]) catch 1;
        count += 1;
        pos += clen;
    }
    return count;
}

fn readMode(allocator: std.mem.Allocator, cfg: *const Config) ![]u8 {
    const runtime_path = try std.fmt.allocPrint(allocator, "{s}/cliphizt/mode", .{cfg.xdg_runtime_dir});
    defer allocator.free(runtime_path);

    if (readModeFile(allocator, runtime_path)) |mode| return mode else |_| {}

    if (cfg.persist_mode) {
        const state_path = try std.fmt.allocPrint(allocator, "{s}/cliphizt/mode", .{cfg.xdg_state_home});
        defer allocator.free(state_path);
        if (readModeFile(allocator, state_path)) |mode| return mode else |_| {}
    }

    return allocator.dupe(u8, "normal");
}

fn readModeFile(allocator: std.mem.Allocator, path: []const u8) ![]u8 {
    const file = try std.Io.Dir.openFileAbsolute(global_io, path, .{});
    defer file.close(global_io);

    var buf: [64]u8 = undefined;
    var reader = std.Io.File.reader(file, global_io, &buf);
    const raw = try reader.interface.allocRemaining(allocator, std.Io.Limit.limited64(32));
    defer allocator.free(raw);

    const trimmed = std.mem.trim(u8, raw, " \t\n\r");
    if (!std.mem.eql(u8, trimmed, "normal") and !std.mem.eql(u8, trimmed, "ephemeral") and !std.mem.eql(u8, trimmed, "single-use")) {
        return error.InvalidMode;
    }
    return allocator.dupe(u8, trimmed);
}

fn unixTimestamp() i64 {
    var ts: std.c.timespec = undefined;
    _ = std.c.clock_gettime(std.c.CLOCK.REALTIME, &ts);
    return ts.sec;
}

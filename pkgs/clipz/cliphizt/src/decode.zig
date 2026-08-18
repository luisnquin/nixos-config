const std = @import("std");
const Db = @import("db.zig").Db;
const Config = @import("config.zig").Config;

const global_io = std.Options.debug_io;

pub fn run(allocator: std.mem.Allocator, cfg: *const Config, args: []const []const u8) !void {
    var input: []u8 = undefined;

    if (args.len > 0) {
        input = try allocator.dupe(u8, args[0]);
    } else {
        var stdin_buf: [65536]u8 = undefined;
        var reader = std.Io.File.reader(.stdin(), global_io, &stdin_buf);
        input = try reader.interface.allocRemaining(allocator, std.Io.Limit.limited64(64 * 1024));
    }
    defer allocator.free(input);

    const line = std.mem.trimEnd(u8, input, "\n\r");
    const id = extractId(line) orelse {
        std.debug.print("error: could not parse ID from input\n", .{});
        std.process.exit(1);
    };

    const db = try Db.open(allocator, cfg.db_path, cfg.preview_width);
    defer db.close();

    db.decodeToStdout(allocator, id) catch |e| switch (e) {
        error.NotFound => {
            std.debug.print("error: item {d} not found\n", .{id});
            std.process.exit(1);
        },
        else => return e,
    };

    const mode = try @import("mode.zig").get(allocator, cfg);
    defer allocator.free(mode);
    if (std.mem.eql(u8, mode, "single-use")) try db.deleteById(allocator, id);
}

fn extractId(line: []const u8) ?i64 {
    if (std.fmt.parseInt(i64, line, 10)) |id| return id else |_| {}
    const tab_pos = std.mem.indexOfScalar(u8, line, '\t') orelse return null;
    return std.fmt.parseInt(i64, line[0..tab_pos], 10) catch null;
}

const std = @import("std");
const Db = @import("db.zig").Db;
const Config = @import("config.zig").Config;

const global_io = std.Options.debug_io;

pub fn run(allocator: std.mem.Allocator, cfg: *const Config) !void {
    const db = try Db.open(allocator, cfg.db_path, cfg.preview_width);
    defer db.close();

    const count = try db.reindex(allocator);

    var buf: [128]u8 = undefined;
    var writer = std.Io.File.writer(.stdout(), global_io, &buf);
    try writer.interface.print("reindexed {d} entries at width {d}\n", .{ count, cfg.preview_width });
    try writer.interface.flush();
}

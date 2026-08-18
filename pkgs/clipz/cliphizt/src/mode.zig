const std = @import("std");
const Config = @import("config.zig").Config;

const global_io = std.Options.debug_io;

pub fn run(allocator: std.mem.Allocator, cfg: *const Config, args: []const []const u8) !void {
    if (args.len == 0 or std.mem.eql(u8, args[0], "get")) {
        const mode = try get(allocator, cfg);
        defer allocator.free(mode);
        try std.Io.File.writeStreamingAll(.stdout(), global_io, mode);
        try std.Io.File.writeStreamingAll(.stdout(), global_io, "\n");
        return;
    }

    if (std.mem.eql(u8, args[0], "toggle")) {
        const current = try get(allocator, cfg);
        defer allocator.free(current);
        const next: []const u8 = if (std.mem.eql(u8, current, "normal"))
            "ephemeral"
        else if (std.mem.eql(u8, current, "ephemeral"))
            "single-use"
        else
            "normal";
        try set(allocator, cfg, next);
        try std.Io.File.writeStreamingAll(.stdout(), global_io, next);
        try std.Io.File.writeStreamingAll(.stdout(), global_io, "\n");
        return;
    }

    if (std.mem.eql(u8, args[0], "set")) {
        if (args.len < 2) {
            std.debug.print("error: mode set requires <normal|ephemeral|single-use>\n", .{});
            std.process.exit(1);
        }
        const mode_str = args[1];
        if (!std.mem.eql(u8, mode_str, "normal") and !std.mem.eql(u8, mode_str, "ephemeral") and !std.mem.eql(u8, mode_str, "single-use")) {
            std.debug.print("error: unknown mode '{s}', use normal, ephemeral or single-use\n", .{mode_str});
            std.process.exit(1);
        }
        try set(allocator, cfg, mode_str);
        return;
    }

    std.debug.print("error: unknown mode subcommand '{s}'\n", .{args[0]});
    std.process.exit(1);
}

pub fn get(allocator: std.mem.Allocator, cfg: *const Config) ![]u8 {
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

pub fn set(allocator: std.mem.Allocator, cfg: *const Config, mode_str: []const u8) !void {
    const runtime_path = try std.fmt.allocPrint(allocator, "{s}/cliphizt/mode", .{cfg.xdg_runtime_dir});
    defer allocator.free(runtime_path);
    try writeModeFile(runtime_path, mode_str);

    if (cfg.persist_mode) {
        const state_path = try std.fmt.allocPrint(allocator, "{s}/cliphizt/mode", .{cfg.xdg_state_home});
        defer allocator.free(state_path);
        try writeModeFile(state_path, mode_str);
    }
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

fn writeModeFile(path: []const u8, mode_str: []const u8) !void {
    if (std.fs.path.dirname(path)) |parent| {
        std.Io.Dir.createDirPath(.cwd(), global_io, parent) catch {};
    }
    const file = try std.Io.Dir.createFileAbsolute(global_io, path, .{ .truncate = true });
    defer file.close(global_io);
    try std.Io.File.writeStreamingAll(file, global_io, mode_str);
    try std.Io.File.writeStreamingAll(file, global_io, "\n");
}

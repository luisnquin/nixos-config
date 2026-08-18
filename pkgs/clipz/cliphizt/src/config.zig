const std = @import("std");
const ttl = @import("ttl.zig");

const io = std.Options.debug_io;

pub const defaults = struct {
    pub const max_items: u64 = 750;
    pub const max_dedupe_search: u64 = 100;
    pub const min_store_length: u64 = 0;
    pub const preview_width: u64 = 100;
    pub const max_store_size: u64 = 5 * 1024 * 1024;
    pub const ephemeral_ttl: u64 = 3600;
    pub const persist_mode: bool = false;
};

pub const Config = struct {
    arena: std.heap.ArenaAllocator,

    max_items: u64,
    max_dedupe_search: u64,
    min_store_length: u64,
    preview_width: u64,
    max_store_size: u64,
    db_path: []const u8,
    ephemeral_ttl: u64,
    persist_mode: bool,

    xdg_cache_home: []const u8,
    xdg_config_home: []const u8,
    xdg_runtime_dir: []const u8,
    xdg_state_home: []const u8,

    pub fn deinit(self: *Config) void {
        self.arena.deinit();
    }
};

fn getenv(name: [*:0]const u8) ?[:0]const u8 {
    return if (std.c.getenv(name)) |p| std.mem.span(p) else null;
}

pub fn load(parent: std.mem.Allocator) !Config {
    var arena = std.heap.ArenaAllocator.init(parent);
    errdefer arena.deinit();
    const a = arena.allocator();

    const home = getenv("HOME") orelse "/tmp";

    const xdg_cache_home = getenv("XDG_CACHE_HOME") orelse
        try std.fmt.allocPrint(a, "{s}/.cache", .{home});
    const xdg_config_home = getenv("XDG_CONFIG_HOME") orelse
        try std.fmt.allocPrint(a, "{s}/.config", .{home});
    const xdg_runtime_dir = getenv("XDG_RUNTIME_DIR") orelse "/tmp";
    const xdg_state_home = getenv("XDG_STATE_HOME") orelse
        try std.fmt.allocPrint(a, "{s}/.local/state", .{home});

    var cfg = Config{
        .arena = arena,
        .max_items = defaults.max_items,
        .max_dedupe_search = defaults.max_dedupe_search,
        .min_store_length = defaults.min_store_length,
        .preview_width = defaults.preview_width,
        .max_store_size = defaults.max_store_size,
        .db_path = try std.fmt.allocPrint(a, "{s}/cliphizt/db", .{xdg_cache_home}),
        .ephemeral_ttl = defaults.ephemeral_ttl,
        .persist_mode = defaults.persist_mode,
        .xdg_cache_home = xdg_cache_home,
        .xdg_config_home = xdg_config_home,
        .xdg_runtime_dir = xdg_runtime_dir,
        .xdg_state_home = xdg_state_home,
    };

    const config_path = getenv("CLIPHIZT_CONFIG_PATH") orelse
        try std.fmt.allocPrint(a, "{s}/cliphizt/config", .{xdg_config_home});

    loadFile(a, &cfg, config_path) catch {};
    applyEnv(a, &cfg) catch {};

    return cfg;
}

fn loadFile(a: std.mem.Allocator, cfg: *Config, path: []const u8) !void {
    const file = std.Io.Dir.openFileAbsolute(io, path, .{}) catch return;
    defer file.close(io);

    var read_buf: [4096]u8 = undefined;
    var reader = std.Io.File.reader(file, io, &read_buf);
    const content = try reader.interface.allocRemaining(a, std.Io.Limit.limited64(64 * 1024));

    var lines = std.mem.splitScalar(u8, content, '\n');
    while (lines.next()) |raw_line| {
        const line = std.mem.trim(u8, raw_line, " \t\r");
        if (line.len == 0 or line[0] == '#') continue;

        const sep = std.mem.indexOfAny(u8, line, " \t") orelse continue;
        const key = std.mem.trim(u8, line[0..sep], " \t");
        const val = std.mem.trim(u8, line[sep..], " \t");
        if (val.len == 0) continue;

        applyKV(a, cfg, key, val) catch {};
    }
}

fn applyEnv(a: std.mem.Allocator, cfg: *Config) !void {
    const entries = [_]struct { env_name: [*:0]const u8, key: []const u8 }{
        .{ .env_name = "CLIPHIZT_MAX_ITEMS", .key = "max-items" },
        .{ .env_name = "CLIPHIZT_MAX_DEDUPE_SEARCH", .key = "max-dedupe-search" },
        .{ .env_name = "CLIPHIZT_MIN_STORE_LENGTH", .key = "min-store-length" },
        .{ .env_name = "CLIPHIZT_PREVIEW_WIDTH", .key = "preview-width" },
        .{ .env_name = "CLIPHIZT_MAX_STORE_SIZE", .key = "max-store-size" },
        .{ .env_name = "CLIPHIZT_DB_PATH", .key = "db-path" },
        .{ .env_name = "CLIPHIZT_EPHEMERAL_TTL", .key = "ephemeral-ttl" },
        .{ .env_name = "CLIPHIZT_PERSIST_MODE", .key = "persist-mode" },
    };
    for (entries) |e| {
        if (getenv(e.env_name)) |val| {
            applyKV(a, cfg, e.key, val) catch {};
        }
    }
}

fn applyKV(a: std.mem.Allocator, cfg: *Config, key: []const u8, val: []const u8) !void {
    if (std.mem.eql(u8, key, "max-items")) {
        cfg.max_items = try std.fmt.parseInt(u64, val, 10);
    } else if (std.mem.eql(u8, key, "max-dedupe-search")) {
        cfg.max_dedupe_search = try std.fmt.parseInt(u64, val, 10);
    } else if (std.mem.eql(u8, key, "min-store-length")) {
        cfg.min_store_length = try std.fmt.parseInt(u64, val, 10);
    } else if (std.mem.eql(u8, key, "preview-width")) {
        cfg.preview_width = try std.fmt.parseInt(u64, val, 10);
    } else if (std.mem.eql(u8, key, "max-store-size")) {
        cfg.max_store_size = try parseSize(val);
    } else if (std.mem.eql(u8, key, "db-path")) {
        cfg.db_path = try a.dupe(u8, val);
    } else if (std.mem.eql(u8, key, "ephemeral-ttl")) {
        cfg.ephemeral_ttl = try ttl.parse(val);
    } else if (std.mem.eql(u8, key, "persist-mode")) {
        cfg.persist_mode = std.mem.eql(u8, val, "true") or std.mem.eql(u8, val, "1");
    }
}

fn parseSize(val: []const u8) !u64 {
    const suffixes = [_]struct { suffix: []const u8, mul: u64 }{
        .{ .suffix = "GiB", .mul = 1024 * 1024 * 1024 },
        .{ .suffix = "MiB", .mul = 1024 * 1024 },
        .{ .suffix = "KiB", .mul = 1024 },
        .{ .suffix = "GB", .mul = 1000 * 1000 * 1000 },
        .{ .suffix = "MB", .mul = 1000 * 1000 },
        .{ .suffix = "KB", .mul = 1000 },
        .{ .suffix = "G", .mul = 1024 * 1024 * 1024 },
        .{ .suffix = "M", .mul = 1024 * 1024 },
        .{ .suffix = "K", .mul = 1024 },
    };
    for (suffixes) |s| {
        if (std.ascii.endsWithIgnoreCase(val, s.suffix)) {
            const n = try std.fmt.parseInt(u64, val[0 .. val.len - s.suffix.len], 10);
            return n * s.mul;
        }
    }
    return try std.fmt.parseInt(u64, val, 10);
}

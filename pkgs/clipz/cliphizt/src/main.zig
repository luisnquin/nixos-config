const std = @import("std");
const config = @import("config.zig");
const store_cmd = @import("store.zig");
const list_cmd = @import("list.zig");
const decode_cmd = @import("decode.zig");
const delete_cmd = @import("delete.zig");
const compact_cmd = @import("compact.zig");
const cleanup_cmd = @import("cleanup.zig");
const reindex_cmd = @import("reindex.zig");
const mode_cmd = @import("mode.zig");

const version = "0.1.0";
const global_io = std.Options.debug_io;

pub fn main(init: std.process.Init) !void {
    const allocator = init.gpa;

    var args_list: std.ArrayList([:0]const u8) = .empty;
    defer args_list.deinit(allocator);

    var it = std.process.Args.Iterator.init(init.minimal.args);
    while (it.next()) |arg| try args_list.append(allocator, arg);

    const args = args_list.items;
    if (args.len < 2) {
        usage();
        std.process.exit(1);
    }

    var cfg = config.load(allocator) catch |e| {
        std.debug.print("error: failed to load config: {}\n", .{e});
        std.process.exit(1);
    };
    defer cfg.deinit();

    const cmd: []const u8 = args[1];
    const rest: []const [:0]const u8 = if (args.len > 2) args[2..] else &.{};
    const str_rest: []const []const u8 = @ptrCast(rest);

    if (std.mem.eql(u8, cmd, "store")) {
        store_cmd.run(allocator, &cfg, str_rest) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "list")) {
        list_cmd.run(allocator, &cfg) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "decode")) {
        decode_cmd.run(allocator, &cfg, str_rest) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "delete")) {
        delete_cmd.runDelete(allocator, &cfg) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "delete-query")) {
        if (rest.len < 1) {
            std.debug.print("error: delete-query requires a query argument\n", .{});
            std.process.exit(1);
        }
        delete_cmd.runDeleteQuery(allocator, &cfg, rest[0]) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "wipe")) {
        delete_cmd.runWipe(allocator, &cfg) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "compact")) {
        compact_cmd.run(allocator, &cfg) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "cleanup")) {
        cleanup_cmd.run(allocator, &cfg) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "reindex")) {
        reindex_cmd.run(allocator, &cfg) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "mode")) {
        mode_cmd.run(allocator, &cfg, str_rest) catch |e| fatal(e);
    } else if (std.mem.eql(u8, cmd, "version")) {
        printVersion(&cfg);
    } else {
        std.debug.print("error: unknown command '{s}'\n\n", .{cmd});
        usage();
        std.process.exit(1);
    }
}

fn fatal(e: anyerror) noreturn {
    std.debug.print("error: {}\n", .{e});
    std.process.exit(1);
}

fn usage() void {
    std.debug.print(
        \\usage: cliphizt <command> [options]
        \\
        \\commands:
        \\  store [--ttl <dur>]            read stdin and store to history
        \\  list                           list history (id<tab>preview, newest first)
        \\  decode [<id>]                  output original data by id (or from stdin)
        \\  delete                         delete entries from stdin
        \\  delete-query <query>           delete entries matching query
        \\  wipe                           delete all entries
        \\  compact                        defragment the database
        \\  cleanup                        delete expired (TTL) entries
        \\  reindex                        re-render stored previews at the current width
        \\  mode get                       print current mode
        \\  mode set <mode>                switch mode
        \\  mode toggle                    cycle modes, print result
        \\  version                        print version and config
        \\
        \\modes:
        \\  normal       keep entries forever (default)
        \\  ephemeral    entries expire after ephemeral-ttl
        \\  single-use   entries are deleted after the first decode
        \\
        \\ttl format: 30s, 5m, 2h, 7d, 1w, 1h30m
        \\
    , .{});
}

fn printVersion(cfg: *const config.Config) void {
    var write_buf: [4096]u8 = undefined;
    var writer = std.Io.File.writer(.stdout(), global_io, &write_buf);
    writer.interface.print(
        \\cliphizt {s}
        \\  max-items:         {d}
        \\  max-dedupe-search: {d}
        \\  min-store-length:  {d}
        \\  preview-width:     {d}
        \\  max-store-size:    {d}
        \\  ephemeral-ttl:     {d}s
        \\  persist-mode:      {}
        \\  db-path:           {s}
        \\
    , .{
        version,
        cfg.max_items,
        cfg.max_dedupe_search,
        cfg.min_store_length,
        cfg.preview_width,
        cfg.max_store_size,
        cfg.ephemeral_ttl,
        cfg.persist_mode,
        cfg.db_path,
    }) catch {};
    writer.interface.flush() catch {};
}

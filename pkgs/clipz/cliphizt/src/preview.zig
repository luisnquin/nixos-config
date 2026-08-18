const std = @import("std");

const ImageFormat = enum { png, jpeg, gif, bmp, tiff, webp };

const ImageInfo = struct {
    format: ImageFormat,
    width: ?u32 = null,
    height: ?u32 = null,
};

fn detectImage(data: []const u8) ?ImageInfo {
    if (data.len < 4) return null;

    if (data.len >= 24 and std.mem.eql(u8, data[0..8], "\x89PNG\r\n\x1a\n")) {
        const width = std.mem.readInt(u32, data[16..20], .big);
        const height = std.mem.readInt(u32, data[20..24], .big);
        return .{ .format = .png, .width = width, .height = height };
    }

    if (data.len >= 2 and data[0] == 0xFF and data[1] == 0xD8) {
        const dims = parseJpegDims(data);
        return .{ .format = .jpeg, .width = dims[0], .height = dims[1] };
    }

    if (data.len >= 10 and
        (std.mem.eql(u8, data[0..6], "GIF87a") or std.mem.eql(u8, data[0..6], "GIF89a")))
    {
        const width = std.mem.readInt(u16, data[6..8], .little);
        const height = std.mem.readInt(u16, data[8..10], .little);
        return .{ .format = .gif, .width = width, .height = height };
    }

    if (data.len >= 26 and data[0] == 'B' and data[1] == 'M') {
        const width = std.mem.readInt(i32, data[18..22], .little);
        const height_raw = std.mem.readInt(i32, data[22..26], .little);
        const height: u32 = @intCast(@abs(height_raw));
        return .{ .format = .bmp, .width = @intCast(@max(0, width)), .height = height };
    }

    if (data.len >= 12 and
        std.mem.eql(u8, data[0..4], "RIFF") and
        std.mem.eql(u8, data[8..12], "WEBP"))
    {
        return .{ .format = .webp };
    }

    if (data.len >= 4) {
        const is_le = data[0] == 'I' and data[1] == 'I';
        const is_be = data[0] == 'M' and data[1] == 'M';
        if (is_le or is_be) {
            const endian: std.builtin.Endian = if (is_le) .little else .big;
            const magic = std.mem.readInt(u16, data[2..4], endian);
            if (magic == 42) return .{ .format = .tiff };
        }
    }

    return null;
}

fn parseJpegDims(data: []const u8) [2]?u32 {
    var i: usize = 2;
    while (i + 4 <= data.len) {
        if (data[i] != 0xFF) break;
        const marker = data[i + 1];
        if (i + 4 > data.len) break;
        const length = std.mem.readInt(u16, data[i + 2 .. i + 4][0..2], .big);
        // SOF markers with dimension info
        switch (marker) {
            0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB => {
                if (i + 9 <= data.len) {
                    const h = std.mem.readInt(u16, data[i + 5 .. i + 7][0..2], .big);
                    const w = std.mem.readInt(u16, data[i + 7 .. i + 9][0..2], .big);
                    return .{ w, h };
                }
            },
            else => {},
        }
        i += 2 + length;
    }
    return .{ null, null };
}

fn fmtSize(size: usize, buf: *[32]u8) []u8 {
    if (size >= 1024 * 1024) {
        return std.fmt.bufPrint(buf, "{d:.1}MiB", .{@as(f64, @floatFromInt(size)) / (1024.0 * 1024.0)}) catch buf[0..0];
    } else if (size >= 1024) {
        return std.fmt.bufPrint(buf, "{d:.1}KiB", .{@as(f64, @floatFromInt(size)) / 1024.0}) catch buf[0..0];
    } else {
        return std.fmt.bufPrint(buf, "{d}B", .{size}) catch buf[0..0];
    }
}

/// One-line summary of an entry, stored alongside it so listings never have to
/// read the payload back. Contains no tab, so callers can prefix `id\t`.
pub fn render(allocator: std.mem.Allocator, data: []const u8, width: u64) ![]u8 {
    if (detectImage(data)) |info| {
        const fmt_name = switch (info.format) {
            .png => "png",
            .jpeg => "jpeg",
            .gif => "gif",
            .bmp => "bmp",
            .tiff => "tiff",
            .webp => "webp",
        };
        var size_buf: [32]u8 = undefined;
        const size_str = fmtSize(data.len, &size_buf);
        if (info.width) |w| {
            if (info.height) |h| {
                return std.fmt.allocPrint(allocator, "[[ binary data {s} {s} {d}x{d} ]]", .{ size_str, fmt_name, w, h });
            }
        }
        return std.fmt.allocPrint(allocator, "[[ binary data {s} {s} ]]", .{ size_str, fmt_name });
    }

    const trimmed = std.mem.trim(u8, data, " \t\n\r");

    // Pre-allocate upper bound: width UTF-8 chars * 4 bytes/char + ellipsis (3 bytes)
    const cap = @min(trimmed.len, width * 4 + 4);
    var buf = try allocator.alloc(u8, cap);
    defer allocator.free(buf);

    var count: u64 = 0;
    var out: usize = 0;
    var pos: usize = 0;
    var truncated = false;
    var last_was_space = false;

    while (pos < trimmed.len) {
        const b = trimmed[pos];

        if (b == '\n' or b == '\r' or b == '\t') {
            if (!last_was_space and out < buf.len) {
                if (count >= width) {
                    truncated = true;
                    break;
                }
                buf[out] = ' ';
                out += 1;
                count += 1;
                last_was_space = true;
            }
            pos += 1;
            continue;
        }

        last_was_space = (b == ' ');
        const clen: usize = std.unicode.utf8ByteSequenceLength(b) catch 1;
        if (count >= width) {
            truncated = true;
            break;
        }
        if (pos + clen <= trimmed.len and out + clen <= buf.len) {
            @memcpy(buf[out .. out + clen], trimmed[pos .. pos + clen]);
            out += clen;
        }
        count += 1;
        pos += clen;
    }

    const text = buf[0..out];
    if (truncated) {
        return std.fmt.allocPrint(allocator, "{s}\u{2026}", .{text});
    }
    return allocator.dupe(u8, text);
}

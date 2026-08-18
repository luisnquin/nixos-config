const std = @import("std");

pub const Error = error{ Empty, InvalidNumber, InvalidUnit };

pub fn parse(input: []const u8) Error!u64 {
    if (input.len == 0) return Error.Empty;

    var total: u64 = 0;
    var i: usize = 0;

    while (i < input.len) {
        const num_start = i;
        while (i < input.len and std.ascii.isDigit(input[i])) : (i += 1) {}
        if (i == num_start) return Error.InvalidNumber;

        const num = std.fmt.parseInt(u64, input[num_start..i], 10) catch return Error.InvalidNumber;
        if (i >= input.len) return Error.InvalidUnit;

        const multiplier: u64 = switch (input[i]) {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            'd' => 86400,
            'w' => 604800,
            else => return Error.InvalidUnit,
        };
        i += 1;
        total += num * multiplier;
    }

    return total;
}

test "parse basic units" {
    const testing = std.testing;
    try testing.expectEqual(@as(u64, 30), try parse("30s"));
    try testing.expectEqual(@as(u64, 300), try parse("5m"));
    try testing.expectEqual(@as(u64, 3600), try parse("1h"));
    try testing.expectEqual(@as(u64, 86400), try parse("1d"));
    try testing.expectEqual(@as(u64, 604800), try parse("1w"));
}

test "parse compound" {
    const testing = std.testing;
    try testing.expectEqual(@as(u64, 5400), try parse("1h30m"));
    try testing.expectEqual(@as(u64, 90061), try parse("1d1h1m1s"));
}

test "parse errors" {
    const testing = std.testing;
    try testing.expectError(Error.Empty, parse(""));
    try testing.expectError(Error.InvalidUnit, parse("5x"));
    try testing.expectError(Error.InvalidNumber, parse("h"));
}

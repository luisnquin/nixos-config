type mullvad_status = {
  server_id : string;
  tunnel_type : string;
  connection : string;
  network_ip : string;
  network_port : string;
  network_proto : string;
  city : string;
  country : string;
}

let capture_stdout command =
  let in_channel = Unix.open_process_in command in
  let rec read_lines acc =
    try
      let line = input_line in_channel in
      read_lines (line :: acc)
    with End_of_file ->
      close_in in_channel;
      List.rev acc
  in
  let result = Array.of_list (read_lines []) in
  result

let remove_parentheses str =
  if
    String.length str >= 2 && str.[0] = '(' && str.[String.length str - 1] = ')'
  then String.sub str 1 (String.length str - 2)
  else str

let get_mullvad_status () =
  let out_lines = capture_stdout "mullvad status -v" in

  let frags1 = out_lines.(0) |> String.split_on_char ' ' in

  let connection =
    let raw_conn = List.nth frags1 0 in
    String.lowercase_ascii raw_conn
  in

  if connection = "connected" then
    let frags2 = out_lines.(1) |> String.split_on_char ':' in

    let network_frags =
      let raw_network = remove_parentheses (List.nth frags1 3) in
      Str.split (Str.regexp "[/:]+") raw_network
    in

    let tunnel_type = String.trim (List.nth frags2 1)
    and city =
      let raw_city = List.nth frags1 5 in
      String.sub raw_city 0 (String.length raw_city - 1)
    in

    {
      server_id = List.nth frags1 2;
      tunnel_type;
      connection;
      network_ip = List.nth network_frags 0;
      network_port = List.nth network_frags 1;
      network_proto = List.nth network_frags 2;
      city;
      country = List.nth frags1 6;
    }
  else
    {
      server_id = "none";
      tunnel_type = "unknown";
      connection;
      network_ip = "none";
      network_port = "none";
      network_proto = "none";
      city = "unknown";
      country = "unknown";
    }

let get_formatted_status out_kind format =
  let status = get_mullvad_status () in

  let out_format = Format.asprintf "%s" format
  and placeholders =
    [
      "{{emoji}}";
      "{{server-id}}";
      "{{city}}";
      "{{country}}";
      "{{connection}}";
      "{{tunnel-type}}";
      "{{network-ip}}";
      "{{network-port}}";
      "{{network-proto}}";
    ]
  and values =
    let emoji =
      match status.connection with
      | "connected" -> "󰠥"
      | "disconnected" -> "󱑘"
      | "disconnecting" -> ""
      | _ -> "󰦅"
    in
    [
      emoji;
      status.server_id;
      status.city;
      status.country;
      status.connection;
      status.tunnel_type;
      status.network_ip;
      status.network_port;
      status.network_proto;
    ]
  in

  let rec replace_placeholders text placeholders replacements =
    match (placeholders, replacements) with
    | [], _ | _, [] -> text
    | ph :: phs, repl :: repls ->
        let text' = Str.global_replace (Str.regexp_string ph) repl text in
        replace_placeholders text' phs repls
  in

  let output_text = replace_placeholders out_format placeholders values in

  match out_kind with
  | "waybar" ->
      let tooltip =
        let tooltip_format =
          "{{country}}, {{city}} - {{server-id}} \
           ({{network-ip}}:{{network-port}}/{{network-proto}})"
        in

        if status.connection = "connected" then
          replace_placeholders tooltip_format placeholders values
        else "Nothing to say."
      in
      Printf.printf "{\"text\": \"%s\", \"tooltip\": \"%s\", \"class\": \"%s\"}"
        output_text tooltip status.connection
  | _ -> Printf.printf "%s\n" output_text

let toggle_mullvad_connection () =
  let status = get_mullvad_status () in

  let code =
    if status.connection = "disconnected" then Sys.command "mullvad connect"
    else Sys.command "mullvad disconnect"
  in

  exit code

let main () =
  if Array.length Sys.argv > 1 then
    match Sys.argv.(1) with
    | "--toggle" | "-t" | "--toggle-connection" -> toggle_mullvad_connection ()
    | "--waybar" ->
        if Array.length Sys.argv > 2 then
          get_formatted_status "waybar" Sys.argv.(2)
        else (* ha *)
          Printf.eprintf "Error: Missing waybar format argument\n"
    | out_format -> get_formatted_status "" out_format
  else Printf.eprintf "Error: Missing command argument\n"

let () = main ()

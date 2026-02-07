{libx, ...}: {
  force = true;
  settings = let
    gmailEntries = map (i: rec {
      keyword = "l${toString i}";
      name = keyword;
      url = "https://mail.google.com/mail/u/${toString i}/#inbox";
      tags = ["gmail"];
    }) [0 1 2 3];
  in
    [
      {
        name = "encore";
        tags = ["encore" "k9"];
        keyword = "encore";
        url = libx.base64.decode "aHR0cHM6Ly9hcHAuZW5jb3JlLmNsb3VkL2dhdGUtazktbXpuaQo=";
      }
      {
        name = "kernel.org";
        url = "https://www.kernel.org";
      }
      {
        name = "orders";
        url = "https://www.aliexpress.com/p/order/index.html";
        keyword = "orders";
      }
      {
        name = "cart";
        url = "https://www.aliexpress.com/p/shoppingcart/index.html";
        keyword = "cart";
      }
      {
        name = "s3";
        url = "https://sa-east-1.console.aws.amazon.com/s3/buckets?region=sa-east-1";
        keyword = "s3";
      }
      {
        name = "iam";
        url = "https://us-east-1.console.aws.amazon.com/iam/home?region=sa-east-1";
        keyword = "iam";
      }
      {
        name = "firebase";
        url = "https://console.firebase.google.com/u/0/";
        keyword = "fire";
      }
      {
        name = "novu";
        url = "https://dashboard-v2.novu.co/env/dev_env_gQwrIEDTzbHL4coa";
        keyword = "novu";
      }
      {
        name = "supabase";
        url = "https://supabase.com/dashboard/project/mjkvxcziwkwohxpuejak";
        keyword = "supa";
      }
      {
        name = "krear3d";
        url = "https://www.tiendakrear3d.com/productos/filamentos/";
        keyword = "3d";
        tags = ["3d"];
      }
      {
        name = "digitalz3d";
        url = "http://www.digitalz3d.com/filamentos";
        keyword = "3dd";
        tags = ["3d"];
      }
    ]
    ++ gmailEntries;
}

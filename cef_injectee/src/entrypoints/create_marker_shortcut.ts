declare global {
    const SISR_PATH: string;
}
const main = async () => {

    const path = SISR_PATH.replace(/\\/g, "/").toLowerCase();
    const working_directory = path.substring(0, path.lastIndexOf("/")).toLowerCase();

    let appID = await SteamClient.Apps.AddShortcut(
        "SISR Marker",
        path,
        working_directory,
        "--marker"
    );

    SteamClient.Apps.RegisterForAppDetails(appID, (dets) => {
        appID = dets.unAppID as number;
    });

    // AddShortcut may or may not set name and params correctly ¯\_(ツ)_/¯
    SteamClient.Apps.SetShortcutLaunchOptions(appID, "--marker");
    SteamClient.Apps.SetShortcutName(appID, "SISR Marker");
    await new Promise((resolve) => setTimeout(resolve, 125));
    // Wait a bit for Steam to process the name change
    // May or may not actually change the shortcutID but whatever, doesn't hurt either
    return appID;
}


__res = new Promise((resolve, reject) => {
    main().then(resolve).catch(reject);
});


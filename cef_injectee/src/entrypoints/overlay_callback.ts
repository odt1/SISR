import { api } from "../lib/api";
const main = async () => {
    const overlayCallback = (
        some_number: number,
        always_0: number,
        overlay_opened_closed_bool: boolean,
        always_true: boolean
    ) => {
        api.overlay(overlay_opened_closed_bool).catch(console.error);
    }
    await (opener as SteamWindow).SteamClient.Overlay.RegisterForOverlayActivated(
        overlayCallback
    );
};

api.ping().then(() => {
    console.log("Ping successful, running main...");
    main();
});
import { api } from "../lib/api";
const main = async () => {
    const overlayCallback = (
        some_number: number,
        always_0: number,
        overlay_opened_closed_bool: boolean,
        always_true: boolean
    ) => {
        api.overlayStateChanged(overlay_opened_closed_bool);
    }
    await (opener as SteamWindow).SteamClient.Overlay.RegisterForOverlayActivated(
        overlayCallback
    );
};

api.connect().then(() => {
    main().catch(console.error);
}).catch(console.error);
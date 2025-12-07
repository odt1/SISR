export const mocks: Record<string, Record<string, unknown | (() => unknown)>> = {
    "http://${SISR_HOST}/test": {
        "GET": () => ({
            message: "This is mocked data!",
            items: [1, 2, 3, 4, 5]
        })
    },
    "http://${SISR_HOST}/ping": {
        "GET": () => ({
            status: "ok"
        })
    }
};

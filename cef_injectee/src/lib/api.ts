async function fetchWithTimeout(url: string, options: RequestInit, timeout: number) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
        const response = await fetch(url, { ...options, signal: controller.signal });
        clearTimeout(timeoutId);
        return response;
    } catch (error) {
        clearTimeout(timeoutId);
        throw error;
    }
}

export const api = {
    async ping(): Promise<boolean> {
        try {
            const response = await fetchWithTimeout(`http://${SISR_HOST}/ping`, { method: 'GET' }, 1000);
            return response.ok;
        } catch {
            return false;
        }
    },

    async overlay(open: boolean): Promise<void> {
        const response = await fetch(`http://${SISR_HOST}/overlay`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ open })
        });

        if (!response.ok) {
            throw new Error(`Failed to set overlay state: ${response.statusText}`);
        }
    }
};

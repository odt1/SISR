import { mocks } from './mocks';

interface XHRWithTracking extends XMLHttpRequest {
    _url?: string;
    _method?: string;
}

function getMockResponse(url: string, method: string): unknown | null {
    const host = window.SISR_HOST || 'http://localhost';

    for (const [pattern, methods] of Object.entries(mocks)) {
        const resolvedPattern = pattern.replace('${SISR_HOST}', host);
        if (url === resolvedPattern || url.startsWith(resolvedPattern)) {
            const mock = methods[method];
            if (!mock) continue;
            return typeof mock === 'function' ? mock() : mock;
        }
    }

    return null;
}

export function setupHttpInterceptor() {
    const originalFetch = window.fetch;
    window.fetch = async function (...args: Parameters<typeof fetch>) {
        const [resource, config] = args;
        const url = typeof resource === 'string' ? resource : (resource instanceof Request ? resource.url : resource.href);
        const method = config?.method || 'GET';

        const mockData = getMockResponse(url, method);
        if (mockData !== null) {
            console.group(`%c🎭 MOCKED ${method} ${url}`, 'color: #FF9800; font-weight: bold');
            console.log('Mock Response:', mockData);
            console.groupEnd();
            return new Response(JSON.stringify(mockData), {
                status: 200,
                headers: { 'Content-Type': 'application/json' }
            });
        }

        console.group(`%c🌐 FETCH ${method} ${url}`, 'color: #4CAF50; font-weight: bold');
        console.log('Request:', { url, method, headers: config?.headers, body: config?.body });

        try {
            const response = await originalFetch(...args);
            const clone = response.clone();
            const contentType = response.headers.get('content-type');
            let data: unknown;

            if (contentType?.includes('application/json')) {
                data = await clone.json();
            } else if (contentType?.includes('text')) {
                data = await clone.text();
            }

            console.log(`Response [${response.status}]:`, { headers: Object.fromEntries(response.headers.entries()), data });
            console.groupEnd();
            return response;
        } catch (error) {
            console.error('Error:', error);
            console.groupEnd();
            throw error;
        }
    };

    const originalOpen = XMLHttpRequest.prototype.open;
    const originalSend = XMLHttpRequest.prototype.send;

    XMLHttpRequest.prototype.open = function (
        this: XHRWithTracking,
        method: string,
        url: string | URL,
        async?: boolean,
        username?: string | null,
        password?: string | null
    ) {
        this._url = url.toString();
        this._method = method;
        return originalOpen.call(this, method, url, async ?? true, username, password);
    };

    XMLHttpRequest.prototype.send = function (this: XHRWithTracking, body?: Document | XMLHttpRequestBodyInit | null) {
        const mockData = getMockResponse(this._url!, this._method!);

        if (mockData !== null) {
            console.group(`%c🎭 MOCKED ${this._method} ${this._url}`, 'color: #FF9800; font-weight: bold');
            console.log('Mock Response:', mockData);
            console.groupEnd();

            setTimeout(() => {
                Object.defineProperty(this, 'status', { value: 200, writable: false });
                Object.defineProperty(this, 'response', { value: JSON.stringify(mockData), writable: false });
                Object.defineProperty(this, 'responseText', { value: JSON.stringify(mockData), writable: false });
                this.dispatchEvent(new Event('load'));
            }, 0);

            return;
        }

        console.group(`%c🌐 XHR ${this._method} ${this._url}`, 'color: #2196F3; font-weight: bold');
        console.log('Request:', { url: this._url, method: this._method, body });

        this.addEventListener('load', () => {
            console.log(`Response [${this.status}]:`, { headers: this.getAllResponseHeaders(), response: this.response });
            console.groupEnd();
        });

        this.addEventListener('error', () => {
            console.error('Error');
            console.groupEnd();
        });

        return originalSend.call(this, body);
    };
}

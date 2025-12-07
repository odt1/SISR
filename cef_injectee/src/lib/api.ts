type WsResponse<T = unknown> = {
    success: boolean;
    data?: T;
    error?: string;
};

type PongResponse = {
    pong: boolean;
    timestamp: number;
};

export const api = {
    ws: undefined as WebSocket | undefined,
    eventHandlers: {} as Record<string, Array<(payload: unknown) => void>>,

    emit(event: string, payload: unknown): void {
        const handlers = this.eventHandlers[event];
        if (handlers) {
            handlers.forEach(handler => {
                try {
                    handler(payload);
                } catch (error) {
                    console.error(`Error in event handler for '${event}':`, error);
                }
            });
        }
    },

    connect(): Promise<PongResponse> {
        return new Promise((resolve, reject) => {
            const socket = new WebSocket(`ws://${SISR_HOST}`);

            socket.onopen = () => {
                this.ws = socket;
                this.ping().then(resolve).catch(reject);
            };

            socket.onerror = () => {
                reject(new Error('WebSocket connection failed'));
            };

            socket.onmessage = (event) => {
                try {
                    const message = JSON.parse(event.data);
                    if (message.type) {
                        this.emit(message.type, message);
                    } else if (message.success !== undefined) {
                        this.emit('response', message);
                    }
                } catch (error) {
                    console.error('Failed to parse WebSocket message:', error);
                }
            };

            socket.onclose = () => {
                this.ws = undefined;
                console.log('WebSocket connection closed');
            };
        });
    },

    ping(): Promise<PongResponse> {
        return new Promise((resolve, reject) => {
            if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
                reject(new Error('WebSocket not connected'));
                return;
            }

            const callback = (payload: WsResponse<PongResponse>) => {
                unsubscribe();
                if (payload.success && payload.data) {
                    resolve(payload.data);
                } else {
                    reject(new Error(payload.error || 'Ping failed'));
                }
            };
            const unsubscribe = this.on<WsResponse<PongResponse>>('response', callback);

            this.ws.send(JSON.stringify({ command: 'ping' }));

            setTimeout(() => {
                unsubscribe();
                reject(new Error('Ping timeout'));
            }, 5000);
        });
    },

    overlayStateChanged(open: boolean): boolean {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
            console.error('WebSocket not connected');
            return false;
        }

        this.ws.send(JSON.stringify({ command: 'overlayStateChanged', open }));
        return true;
    },

    on<T = unknown>(event: string, callback: (payload: T) => void): () => void {
        if (!this.eventHandlers[event]) {
            this.eventHandlers[event] = [];
        }
        this.eventHandlers[event].push(callback as (payload: unknown) => void);

        return () => {
            const handlers = this.eventHandlers[event];
            if (handlers) {
                const index = handlers.indexOf(callback as (payload: unknown) => void);
                if (index > -1) {
                    handlers.splice(index, 1);
                }
                if (handlers.length === 0) {
                    delete this.eventHandlers[event];
                }
            }
        };
    },

    disconnect(): void {
        if (this.ws) {
            this.ws.close();
            this.ws = undefined;
        }
    }
};

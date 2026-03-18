declare module "react-native-zeroconf" {
    export interface ZeroconfService {
        name: string;
        fullName: string;
        addresses: string[];
        host: string;
        port: number;
        txt: Record<string, string>;
    }

    export default class Zeroconf {
        on(event: "resolved", handler: (service: ZeroconfService) => void): this;
        on(event: "found", handler: (name: string) => void): this;
        on(event: "remove", handler: (name: string) => void): this;
        on(event: "error", handler: (err: Error) => void): this;
        on(event: "start" | "stop" | "update", handler: () => void): this;
        scan(type?: string, protocol?: string, domain?: string): void;
        stop(): void;
        removeAllListeners(): void;
        getServices(): Record<string, ZeroconfService>;
        publishService(type: string, protocol: string, domain: string, name: string, port: number, txt?: Record<string, string>): void;
        unpublishService(name: string): void;
    }
}

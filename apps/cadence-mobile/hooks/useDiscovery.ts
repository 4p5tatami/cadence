import { useEffect, useState } from "react";
import Zeroconf from 'react-native-zeroconf';

export interface DiscoveredDevice {
    name: string;
    url: string;
}

export function useDiscovery() {
    const [devices, setDevices] = useState<DiscoveredDevice[]>([]);

    useEffect(() => {
        const zc = new Zeroconf();

        zc.on("resolved", (service: { addresses: any[]; port: any; name: string; }) => {
            const ip = service.addresses?.[0];
            if (!ip) return;
            const url = `ws://${ip}:${service.port}`;
            setDevices((prev) => {
                if (prev.find((d) => d.name === service.name)) return prev;
                return [...prev, { name: service.name, url }];
            });
        });

        zc.on("remove", (name: string) => {
            setDevices((prev) => prev.filter((d) => d.name !== name));
        });

        zc.scan("cadence", "tcp", "local.");

        return () => {
            zc.stop();
            zc.removeAllListeners();
        };
    }, []);

    return devices;
}

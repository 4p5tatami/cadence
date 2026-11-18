import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useDesktopSync } from "@/hooks/useDesktopSync";
import { useDiscovery, type DiscoveredDevice } from "@/hooks/useDiscovery";
import { useToast } from "@/providers/ToastProvider";

type DesktopSync = ReturnType<typeof useDesktopSync>;

interface CadenceContextValue extends DesktopSync {
    /** Devices advertising `_cadence._tcp` on the local network. */
    devices: DiscoveredDevice[];
    /** Clears the device list and restarts the network scan. */
    rescan: () => void;
    /** The device we are connected (or connecting) to. */
    device: DiscoveredDevice | null;
    /** Last connection error, cleared on the next connect attempt. */
    error: string | null;
    connect: (device: DiscoveredDevice) => void;
    disconnect: () => void;
}

const CadenceContext = createContext<CadenceContextValue | null>(null);

/**
 * Owns the single desktop connection and discovery scan so every screen
 * (connect, search, now playing, mini player) shares the same state.
 */
export function CadenceProvider({ children }: { children: ReactNode }) {
    const [device, setDevice] = useState<DiscoveredDevice | null>(null);
    const [error, setError] = useState<string | null>(null);

    const discovery = useDiscovery();
    const devices = discovery.devices;

    const rescan = useCallback(() => {
        setError(null);
        discovery.rescan();
    }, [discovery.rescan]);
    const sync = useDesktopSync(device?.url ?? null);
    const toast = useToast();

    // Surface commands the desktop rejected (e.g. a track whose file is missing).
    useEffect(() => {
        if (sync.lastError) toast.show(sync.lastError.message);
    }, [sync.lastError, toast]);

    // When a connection fails (or retries run out), drop the device so the user can
    // retry, and toast it since they may be on another screen.
    const wasConnected = useRef(false);
    useEffect(() => {
        if (sync.status === "connected") wasConnected.current = true;
        if (sync.status === "error") {
            const name = device?.name ?? "the desktop";
            const message = wasConnected.current ? `Lost connection to ${name}.` : `Couldn't connect to ${name}.`;
            wasConnected.current = false;
            setDevice(null);
            setError(message);
            toast.show(message);
        }
        // Only react to status transitions; `device` is read for its name.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [sync.status]);

    const connect = useCallback((d: DiscoveredDevice) => {
        wasConnected.current = false;
        setError(null);
        setDevice(d);
    }, []);

    const disconnect = useCallback(() => {
        setError(null);
        setDevice(null);
    }, []);

    const value = useMemo<CadenceContextValue>(
        () => ({ ...sync, devices, rescan, device, error, connect, disconnect }),
        [sync, devices, rescan, device, error, connect, disconnect],
    );

    return <CadenceContext.Provider value={value}>{children}</CadenceContext.Provider>;
}

export function useCadence() {
    const ctx = useContext(CadenceContext);
    if (!ctx) throw new Error("useCadence must be used inside <CadenceProvider>");
    return ctx;
}

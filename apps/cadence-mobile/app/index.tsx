import { Redirect } from "expo-router";

/** Entry point: start on the connect screen. */
export default function Index() {
    return <Redirect href="/connect" />;
}

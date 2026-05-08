import { useEffect, useRef } from 'react';
import { StatusBar } from 'expo-status-bar';
import { NavigationContainer, DarkTheme } from '@react-navigation/native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import {
  AccountScreen,
  CreditsInfoScreen,
  ExportDataScreen,
  LoomInfoScreen,
  PrivacyScreen,
} from './src/screens/you/YouDetailScreens';
import { AnkyverseTrailScreen } from './src/screens/AnkyverseTrailScreen';
import { AuthScreen } from './src/screens/AuthScreen';
import { CreditsScreen } from './src/screens/CreditsScreen';
import { DayChamberScreen } from './src/screens/DayChamberScreen';
import { LoomScreen } from './src/screens/LoomScreen';
import { OnboardingScreen } from './src/screens/OnboardingScreen';
import { PastScreen } from './src/screens/PastScreen';
import { RevealScreen } from './src/screens/RevealScreen';
import { TodayScreen } from './src/screens/TodayScreen';
import { ThreadScreen } from './src/screens/ThreadScreen';
import { TrackScreen } from './src/screens/TrackScreen';
import { TrailScreen } from './src/screens/TrailScreen';
import { YouScreen } from './src/screens/YouScreen';
import { WriteRootScreen } from './src/screens/WriteRootScreen';
import { WriteScreen } from './src/screens/WriteScreen';
import { AnkyPrivyProvider } from './src/lib/privy/PrivyProvider';
import { useAnkyPrivyWallet } from './src/lib/privy/useAnkyPrivyWallet';
import { AuthModalProvider } from './src/auth/AuthModalContext';
import { AnkyPresenceProvider } from './src/presence/AnkyPresenceContext';
import { AnkyPresenceOverlay } from './src/presence/AnkyPresenceOverlay';
import { ankyColors } from './src/theme/tokens';
import type { ThreadMode } from './src/lib/thread/types';

export type RootStackParamList = {
  Write: { replayOnboarding?: boolean } | undefined;
  Track: undefined;
  You: undefined;
  Account: undefined;
  Privacy: undefined;
  ExportData: undefined;
  CreditsInfo: undefined;
  LoomInfo: undefined;
  ActiveWriting:
    | {
        dayNumber?: number;
        isoDate?: string;
        recoverDraft?: boolean;
        sessionKind?: "daily_seal" | "extra_thread";
        sojourn?: number;
      }
    | undefined;
  Today: undefined;
  Onboarding: undefined;
  Auth: undefined;
  Loom: undefined;
  Trail: undefined;
  DayChamber: { day: number };
  AnkyverseTrail: undefined;
  Past: undefined;
  Reveal: { fileName?: string } | undefined;
  Thread: {
    sessionHash: string;
    mode?: ThreadMode;
    source?: "entry" | "reflection" | "reveal";
  };
  Credits:
    | {
        fileName?: string;
        processingType?: "reflection" | "image" | "full_anky" | "deep_mirror" | "full_sojourn_archive";
      }
    | undefined;
  Entry: { fileName: string };
};

const Stack = createNativeStackNavigator<RootStackParamList>();

const navigationTheme = {
  ...DarkTheme,
  colors: {
    ...DarkTheme.colors,
    background: ankyColors.bg,
    border: ankyColors.border,
    card: ankyColors.bg,
    primary: ankyColors.gold,
    text: ankyColors.text,
  },
};

export default function App() {
  return (
    <SafeAreaProvider>
      <AnkyPrivyProvider>
        <AuthModalProvider>
          <EmbeddedWalletBootstrapper />
          <AnkyPresenceProvider>
            <NavigationContainer theme={navigationTheme}>
              <StatusBar style="light" />
              <Stack.Navigator
                initialRouteName="Write"
                screenOptions={{
                  contentStyle: { backgroundColor: ankyColors.bg },
                  headerShadowVisible: false,
                  headerStyle: { backgroundColor: ankyColors.bg },
                  headerTintColor: ankyColors.gold,
                  headerTitleStyle: { color: ankyColors.text },
                }}
              >
              <Stack.Screen
                component={WriteRootScreen}
                name="Write"
                options={{ animation: "none", headerShown: false }}
              />
              <Stack.Screen
                component={TrackScreen}
                name="Track"
                options={{ animation: "none", headerShown: false }}
              />
              <Stack.Screen
                component={YouScreen}
                name="You"
                options={{ animation: "none", headerShown: false }}
              />
              <Stack.Screen
                component={AccountScreen}
                name="Account"
                options={{ animation: "slide_from_right", gestureEnabled: true, headerShown: false }}
              />
              <Stack.Screen
                component={PrivacyScreen}
                name="Privacy"
                options={{ animation: "slide_from_right", gestureEnabled: true, headerShown: false }}
              />
              <Stack.Screen
                component={ExportDataScreen}
                name="ExportData"
                options={{ animation: "slide_from_right", gestureEnabled: true, headerShown: false }}
              />
              <Stack.Screen
                component={CreditsInfoScreen}
                name="CreditsInfo"
                options={{ animation: "slide_from_right", gestureEnabled: true, headerShown: false }}
              />
              <Stack.Screen
                component={LoomInfoScreen}
                name="LoomInfo"
                options={{ animation: "slide_from_right", gestureEnabled: true, headerShown: false }}
              />
              <Stack.Screen
                component={WriteScreen}
                name="ActiveWriting"
                options={{ animation: "none", headerShown: false }}
              />
              <Stack.Screen component={TodayScreen} name="Today" options={{ headerShown: false }} />
              <Stack.Screen
                component={OnboardingScreen}
                name="Onboarding"
                options={{ headerShown: false }}
              />
              <Stack.Screen component={AuthScreen} name="Auth" options={{ headerShown: false }} />
              <Stack.Screen component={LoomScreen} name="Loom" options={{ headerShown: false }} />
              <Stack.Screen component={TrailScreen} name="Trail" options={{ headerShown: false }} />
              <Stack.Screen
                component={DayChamberScreen}
                name="DayChamber"
                options={{ headerShown: false }}
              />
              <Stack.Screen
                component={AnkyverseTrailScreen}
                name="AnkyverseTrail"
                options={{ headerShown: false }}
              />
              <Stack.Screen component={RevealScreen} name="Reveal" options={{ headerShown: false }} />
              <Stack.Screen component={ThreadScreen} name="Thread" options={{ headerShown: false }} />
              <Stack.Screen component={PastScreen} name="Past" options={{ title: 'Local Archive' }} />
              <Stack.Screen component={CreditsScreen} name="Credits" options={{ title: 'Credits' }} />
              <Stack.Screen component={RevealScreen} name="Entry" options={{ headerShown: false }} />
              </Stack.Navigator>
            </NavigationContainer>
            <AnkyPresenceOverlay />
          </AnkyPresenceProvider>
        </AuthModalProvider>
      </AnkyPrivyProvider>
    </SafeAreaProvider>
  );
}

function EmbeddedWalletBootstrapper() {
  const wallet = useAnkyPrivyWallet();
  const attemptedRef = useRef(false);

  useEffect(() => {
    if (!wallet.authenticated) {
      attemptedRef.current = false;
      return;
    }

    if (
      attemptedRef.current ||
      wallet.hasWallet ||
      !wallet.canCreateEmbeddedWallet
    ) {
      return;
    }

    attemptedRef.current = true;
    void wallet.createWallet().catch((error: unknown) => {
      console.warn("Embedded Solana wallet bootstrap failed.", error);
    });
  }, [
    wallet.authenticated,
    wallet.canCreateEmbeddedWallet,
    wallet.createWallet,
    wallet.hasWallet,
  ]);

  return null;
}

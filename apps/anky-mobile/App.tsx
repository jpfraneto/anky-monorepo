import { StatusBar } from 'expo-status-bar';
import { NavigationContainer, DarkTheme } from '@react-navigation/native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { AnkyverseTrailScreen } from './src/screens/AnkyverseTrailScreen';
import { CreditsScreen } from './src/screens/CreditsScreen';
import { EntryScreen } from './src/screens/EntryScreen';
import { LoomScreen } from './src/screens/LoomScreen';
import { PastScreen } from './src/screens/PastScreen';
import { RevealScreen } from './src/screens/RevealScreen';
import { WriteScreen } from './src/screens/WriteScreen';
import { ankyColors } from './src/theme/tokens';

export type RootStackParamList = {
  Loom: undefined;
  AnkyverseTrail: undefined;
  Write:
    | {
        dayNumber?: number;
        isoDate?: string;
        recoverDraft?: boolean;
        sojourn?: number;
      }
    | undefined;
  Past: undefined;
  Reveal: { fileName?: string } | undefined;
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
      <NavigationContainer theme={navigationTheme}>
        <StatusBar style="light" />
        <Stack.Navigator
          initialRouteName="Loom"
          screenOptions={{
            contentStyle: { backgroundColor: ankyColors.bg },
            headerShadowVisible: false,
            headerStyle: { backgroundColor: ankyColors.bg },
            headerTintColor: ankyColors.gold,
            headerTitleStyle: { color: ankyColors.text },
          }}
        >
          <Stack.Screen component={LoomScreen} name="Loom" options={{ headerShown: false }} />
          <Stack.Screen
            component={AnkyverseTrailScreen}
            name="AnkyverseTrail"
            options={{ headerShown: false }}
          />
          <Stack.Screen component={WriteScreen} name="Write" options={{ headerShown: false }} />
          <Stack.Screen component={RevealScreen} name="Reveal" options={{ headerShown: false }} />
          <Stack.Screen component={PastScreen} name="Past" options={{ title: 'Local Archive' }} />
          <Stack.Screen component={CreditsScreen} name="Credits" options={{ title: 'Credits' }} />
          <Stack.Screen component={EntryScreen} name="Entry" options={{ title: '.anky' }} />
        </Stack.Navigator>
      </NavigationContainer>
    </SafeAreaProvider>
  );
}

clear
osascript -e 'display notification "Build Started"'
yarn build-native
osascript -e 'display notification "Build Complete"'

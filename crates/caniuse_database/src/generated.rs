use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum BrowserAgent {
  /// Microsoft Internet Explorer
  Ie,
  /// Microsoft Edge
  Edge,
  /// Mozilla Firefox
  Firefox,
  /// Google Chrome
  Chrome,
  /// Safari
  Safari,
  /// Opera
  Opera,
  /// Safari on iOS
  IosSaf,
  /// Opera Mini
  OpMini,
  /// Android Browser / Webview
  Android,
  ///
  Bb,
  /// Opera for Android
  OpMob,
  /// Google Chrome for Android
  AndChr,
  /// Mozilla Firefox for Android
  AndFf,
  ///
  IeMob,
  /// UC Browser for Android
  AndUc,
  /// Samsung Internet Browser
  Samsung,
  /// QQ Browser for Android
  AndQq,
  /// Baidu Browser for Android
  Baidu,
  /// KaiOS Browser
  Kaios,
  /// Any other browser
  Any(String),
}
impl BrowserAgent {
  pub fn key(&self) -> &str {
    match self {
      BrowserAgent::Ie => "ie",
      BrowserAgent::Edge => "edge",
      BrowserAgent::Firefox => "firefox",
      BrowserAgent::Chrome => "chrome",
      BrowserAgent::Safari => "safari",
      BrowserAgent::Opera => "opera",
      BrowserAgent::IosSaf => "ios_saf",
      BrowserAgent::OpMini => "op_mini",
      BrowserAgent::Android => "android",
      BrowserAgent::Bb => "bb",
      BrowserAgent::OpMob => "op_mob",
      BrowserAgent::AndChr => "and_chr",
      BrowserAgent::AndFf => "and_ff",
      BrowserAgent::IeMob => "ie_mob",
      BrowserAgent::AndUc => "and_uc",
      BrowserAgent::Samsung => "samsung",
      BrowserAgent::AndQq => "and_qq",
      BrowserAgent::Baidu => "baidu",
      BrowserAgent::Kaios => "kaios",
      BrowserAgent::Any(key) => key,
    }
  }
  pub fn from_key(key: &str) -> Self {
    match key {
      "ie" => BrowserAgent::Ie,
      "edge" => BrowserAgent::Edge,
      "firefox" => BrowserAgent::Firefox,
      "chrome" => BrowserAgent::Chrome,
      "safari" => BrowserAgent::Safari,
      "opera" => BrowserAgent::Opera,
      "ios_saf" => BrowserAgent::IosSaf,
      "op_mini" => BrowserAgent::OpMini,
      "android" => BrowserAgent::Android,
      "bb" => BrowserAgent::Bb,
      "op_mob" => BrowserAgent::OpMob,
      "and_chr" => BrowserAgent::AndChr,
      "and_ff" => BrowserAgent::AndFf,
      "ie_mob" => BrowserAgent::IeMob,
      "and_uc" => BrowserAgent::AndUc,
      "samsung" => BrowserAgent::Samsung,
      "and_qq" => BrowserAgent::AndQq,
      "baidu" => BrowserAgent::Baidu,
      "kaios" => BrowserAgent::Kaios,
      key => BrowserAgent::Any(key.to_string()),
    }
  }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum BrowserFeature {
  /// AAC audio file format
  ///
  /// Advanced Audio Coding format, designed to be the successor format to MP3, with generally better sound quality.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Advanced_Audio_Coding)
  Aac,
  /// AbortController & AbortSignal
  ///
  /// Controller object that allows you to abort one or more DOM requests made with the Fetch API.
  ///
  /// * [Abortable Fetch - Google Developers article](https://developers.google.com/web/updates/2017/09/abortable-fetch)
  /// * [AbortController - MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/API/AbortController)
  /// * [AbortSignal - MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal)
  Abortcontroller,
  /// Accelerometer
  ///
  /// Defines `Accelerometer`, `LinearAccelerationSensor` and `GravitySensor` interfaces for obtaining information about acceleration applied to the X, Y and Z axis of a device that hosts the sensor.
  ///
  /// * [Demo](https://intel.github.io/generic-sensor-demos/punchmeter/)
  /// * [Article](https://developers.google.com/web/updates/2017/09/sensors-for-the-web#acceleration-and-linear-accelerometer-sensor)
  Accelerometer,
  /// EventTarget.addEventListener()
  ///
  /// The modern standard API for adding DOM event handlers. Introduced in the DOM Level 2 Events spec. Also implies support for `removeEventListener`, the [capture phase](https://dom.spec.whatwg.org/#dom-event-capturing_phase) of DOM event dispatch, as well as the `stopPropagation()` and `preventDefault()` event methods.
  ///
  /// * [MDN Web Docs - addEventListener](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener)
  /// * [Financial Times IE8 polyfill](https://github.com/Financial-Times/polyfill-service/blob/master/polyfills/Event/polyfill.js)
  /// * [WebReflection ie8 polyfill](https://github.com/WebReflection/ie8)
  Addeventlistener,
  /// Ambient Light Sensor
  ///
  /// Defines a concrete sensor interface to monitor the ambient light level or illuminance of the device’s environment.
  ///
  /// * [Demo](https://intel.github.io/generic-sensor-demos/ambient-map/build/bundled/)
  /// * [Article](https://developers.google.com/web/updates/2017/09/sensors-for-the-web)
  /// * [MDN Web Docs - Ambient Light Sensor](https://developer.mozilla.org/en-US/docs/Web/API/Ambient_Light_Sensor_API)
  AmbientLight,
  /// Animated PNG (APNG)
  ///
  /// Like animated GIFs, but allowing 24-bit colors and alpha transparency
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/APNG)
  /// * [Polyfill using canvas](https://github.com/davidmz/apng-canvas)
  /// * [Chrome extension providing support](https://chrome.google.com/webstore/detail/ehkepjiconegkhpodgoaeamnpckdbblp)
  /// * [Chromium issue (fixed)](https://code.google.com/p/chromium/issues/detail?id=437662)
  Apng,
  /// Array.prototype.find
  ///
  /// The `find()` method returns the value of the first item in the array based on the result of the provided testing function.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/find)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-array)
  ArrayFind,
  /// Array.prototype.findIndex
  ///
  /// The `findIndex()` method returns the index of the first element in the array that satisfies the provided testing function.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/findIndex)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-array)
  ArrayFindIndex,
  /// flat & flatMap array methods
  ///
  /// Methods to flatten any sub-arrays found in an array by concatenating their elements.
  ///
  /// * [MDN article on Array.prototype.flat](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/flat)
  /// * [MDN article on Array.prototype.flatMap](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/flatMap)
  /// * [Article on the history of the `flat` methods](https://developers.google.com/web/updates/2018/03/smooshgate)
  /// * [Polyfill for flat & flatMap](https://github.com/jonathantneal/array-flat-polyfill)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-array)
  ArrayFlat,
  /// Array.prototype.includes
  ///
  /// Determines whether or not an array includes the given value, returning a boolean value (unlike `indexOf`).
  ///
  /// * [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/includes#Browser_compatibility)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-array)
  ArrayIncludes,
  /// Arrow functions
  ///
  /// Function shorthand using `=>` syntax and lexical `this` binding.
  ///
  /// * [ECMAScript 6 features: Arrows](https://github.com/lukehoban/es6features#arrows)
  /// * [MDN Web Docs - Arrow functions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Functions/Arrow_functions)
  ArrowFunctions,
  /// asm.js
  ///
  /// An extraordinarily optimizable, low-level subset of JavaScript, intended to be a compile target from languages like C++.
  ///
  /// * [Homepage](http://asmjs.org/)
  /// * [Source for spec and tools](https://github.com/dherman/asm.js/)
  /// * [Bringing Asm.js to Chakra and Microsoft Edge](https://blogs.windows.com/msedgedev/2015/05/07/bringing-asm-js-to-chakra-microsoft-edge/)
  /// * [Microsoft Edge support announcement](https://dev.modern.ie/platform/changelog/10532-pc/)
  /// * [MDN article about asm.js](https://developer.mozilla.org/en-US/docs/Games/Tools/asm.js)
  /// * [Wikipedia article about asm.js](https://en.wikipedia.org/wiki/Asm.js)
  Asmjs,
  /// Asynchronous Clipboard API
  ///
  /// A modern, asynchronous Clipboard API based on Promises
  ///
  /// * [MDN Web Docs - Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API)
  /// * [W3C Async Clipboard Explainer](https://github.com/w3c/clipboard-apis/blob/master/explainer.adoc)
  /// * [Unlocking Clipboard Access](https://web.dev/async-clipboard/)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1619251)
  AsyncClipboard,
  /// Async functions
  ///
  /// Async functions make it possible to treat functions returning Promise objects as if they were synchronous.
  ///
  /// * [MDN Web Docs - Async functions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/async_function)
  /// * [Async functions - making promises friendly](https://developers.google.com/web/fundamentals/getting-started/primers/async-functions)
  AsyncFunctions,
  /// Base64 encoding and decoding
  ///
  /// Utility functions for encoding and decoding strings to and from base 64: window.atob() and window.btoa().
  ///
  /// * [MDN Web Docs - btoa()](https://developer.mozilla.org/en-US/docs/Web/API/Window.btoa)
  /// * [MDN Web Docs - atob()](https://developer.mozilla.org/en-US/docs/Web/API/Window.atob)
  /// * [Polyfill](https://github.com/davidchambers/Base64.js)
  AtobBtoa,
  /// Audio element
  ///
  /// Method of playing sound on webpages (without requiring a plug-in). Includes support for the following media properties: `currentSrc`, `currentTime`, `paused`, `playbackRate`, `buffered`, `duration`, `played`, `seekable`, `ended`, `autoplay`, `loop`, `controls`, `volume` & `muted`
  ///
  /// * [HTML5 Doctor article](https://html5doctor.com/native-audio-in-the-browser/)
  /// * [Detailed article on video/audio elements](https://dev.opera.com/articles/everything-you-need-to-know-html5-video-audio/)
  /// * [Demos of audio player that uses the audio element](https://www.jplayer.org/latest/demos/)
  /// * [Detailed article on support](https://24ways.org/2010/the-state-of-html5-audio)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/audio.js#audio)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/audio)
  /// * [The State of HTML5 Audio](https://www.phoboslab.org/log/2011/03/the-state-of-html5-audio)
  Audio,
  /// Web Audio API
  ///
  /// High-level JavaScript API for processing and synthesizing audio
  ///
  /// * [Polyfill to support Web Audio API in Firefox](https://github.com/corbanbrook/audionode.js)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/webaudio)
  /// * [Polyfill to enable Web Audio API through Firefox Audio Data api or flash](https://github.com/g200kg/WAAPISim)
  /// * [MDN Web Docs - Web Audio API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API)
  AudioApi,
  /// Audio Tracks
  ///
  /// Method of specifying and selecting between multiple audio tracks. Useful for providing audio descriptions, director's commentary, additional languages, alternative takes, etc.
  ///
  /// * [MDN Web Docs - HTMLMediaElement.audioTracks](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/audioTracks)
  Audiotracks,
  /// Autofocus attribute
  ///
  /// Allows a form field to be immediately focused on page load.
  ///
  /// * [Article on autofocus](https://davidwalsh.name/autofocus)
  /// * [MDN Web Docs - autofocus attribute](https://developer.mozilla.org/en/docs/Web/HTML/Element/input#attr-autofocus)
  Autofocus,
  /// Auxclick
  ///
  /// The click event for non-primary buttons of input devices
  ///
  /// * [MDN Web Docs - auxclick](https://developer.mozilla.org/en-US/docs/Web/Events/auxclick)
  /// * [Firefox implementation](https://bugzilla.mozilla.org/show_bug.cgi?id=1304044)
  /// * [WebKit bug](https://bugs.webkit.org/show_bug.cgi?id=22382)
  /// * [Original Proposal](https://wicg.github.io/auxclick/)
  Auxclick,
  /// AV1 video format
  ///
  /// AV1 (AOMedia Video 1) is a royalty-free video format by the Alliance for Open Media, meant to succeed its predecessor VP9 and compete with the [HEVC/H.265](/hevc) format.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/AV1)
  /// * [Sample video from Bitmovin](https://bitmovin.com/demos/av1)
  /// * [Sample video from Facebook](https://www.facebook.com/330716120785217/videos/330723190784510/)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1452683)
  /// * [Safari implementation bug](https://bugs.webkit.org/show_bug.cgi?id=207547)
  Av1,
  /// AVIF image format
  ///
  /// A modern image format based on the [AV1 video format](/av1). AVIF generally has better compression than [WebP](/webp), JPEG, PNG and GIF and is designed to supersede them. AVIF competes with [JPEG XL](/jpegxl) which has similar compression quality and is generally seen as more feature-rich than AVIF.
  ///
  /// * [Polyfill](https://github.com/Kagami/avif.js)
  /// * [AVIF for Next-Generation Image Coding - blog post](https://netflixtechblog.com/avif-for-next-generation-image-coding-b1d75675fe4)
  /// * [Safari support bug](https://bugs.webkit.org/show_bug.cgi?id=207750)
  /// * [Firefox support bug for image sequence and animation](https://bugzilla.mozilla.org/show_bug.cgi?id=1686338)
  Avif,
  /// CSS background-attachment
  ///
  /// Method of defining how a background image is attached to a scrollable element. Values include `scroll` (default), `fixed` and `local`.
  ///
  /// * [MDN Web Docs - background-attachment](https://developer.mozilla.org/en-US/docs/Web/CSS/background-attachment)
  BackgroundAttachment,
  /// Background-clip: text
  ///
  /// Clipping a background image to the foreground text.
  ///
  /// * [[css-backgrounds] Standardize 'background-clip: text'](https://lists.w3.org/Archives/Public/www-style/2016Mar/0283.html)
  /// * [CSS Backgrounds and Borders Module Level 4](https://drafts.csswg.org/css-backgrounds-4/#background-clip)
  /// * [MDN Web Docs - background-clip](https://developer.mozilla.org/en-US/docs/Web/CSS/background-clip)
  BackgroundClipText,
  /// CSS3 Background-image options
  ///
  /// New properties to affect background images, including background-clip, background-origin and background-size
  ///
  /// * [Detailed compatibility tables and demos](http://www.standardista.com/css3/css3-background-properties)
  /// * [Polyfill for IE7-8](https://github.com/louisremi/background-size-polyfill)
  /// * [MDN Web Docs - background-image](https://developer.mozilla.org/en/docs/Web/CSS/background-image)
  BackgroundImgOpts,
  /// background-position-x & background-position-y
  ///
  /// CSS longhand properties to define x or y positions separately.
  ///
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=550426)
  /// * [Blog post on background-position-x & y properties](https://snook.ca/archives/html_and_css/background-position-x-y)
  /// * [MDN Web Docs - background-position-x](https://developer.mozilla.org/en-US/docs/Web/CSS/background-position-x)
  /// * [MDN Web Docs - background-position-y](https://developer.mozilla.org/en/docs/Web/CSS/background-position-y)
  BackgroundPositionXY,
  /// CSS background-repeat round and space
  ///
  /// Allows CSS background images to be repeated without clipping.
  ///
  /// * [MDN Web Docs - background-repeat](https://developer.mozilla.org//docs/Web/CSS/background-repeat)
  /// * [CSS-Tricks article on background-repeat](https://css-tricks.com/almanac/properties/b/background-repeat/)
  BackgroundRepeatRoundSpace,
  /// Background Sync API
  ///
  /// Provides one-off and periodic synchronization for Service Workers with an onsync event.
  ///
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1217544)
  /// * [SyncManager on MDN Web Docs](https://developer.mozilla.org/docs/Web/API/SyncManager)
  /// * [Google Developers blog: Introducing Background Sync](https://developers.google.com/web/updates/2015/12/background-sync)
  BackgroundSync,
  /// Battery Status API
  ///
  /// Method to provide information about the battery status of the hosting device.
  ///
  /// * [MDN Web Docs - battery status](https://developer.mozilla.org/en-US/docs/WebAPI/Battery_Status)
  /// * [Simple demo](https://pazguille.github.io/demo-battery-api/)
  BatteryStatus,
  /// Beacon API
  ///
  /// Allows data to be sent asynchronously to a server with `navigator.sendBeacon`, even after a page was closed. Useful for posting analytics data the moment a user was finished using the page.
  ///
  /// * [MDN Web Docs - Beacon](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)
  Beacon,
  /// Printing Events
  ///
  /// Window fires `beforeprint` and `afterprint` events so the printed document can be annotated.
  ///
  /// * [MDN Web Docs - Detecting print requests](https://developer.mozilla.org/en-US/docs/Web/Guide/Printing#Detecting_print_requests)
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=218205)
  /// * [Safari support bug](https://bugs.webkit.org/show_bug.cgi?id=19937)
  Beforeafterprint,
  /// BigInt
  ///
  /// Arbitrary-precision integers in JavaScript.
  ///
  /// * [GitHub repository](https://github.com/tc39/proposal-bigint)
  /// * [Blog article from Google Developer](https://developers.google.com/web/updates/2018/05/bigint)
  /// * [Blog article from Dr. Axel Rauschmayer](https://2ality.com/2017/03/es-integer.html)
  /// * [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/BigInt)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1366287)
  Bigint,
  /// Blob constructing
  ///
  /// Construct Blobs (binary large objects) either using the BlobBuilder API (deprecated) or the Blob constructor.
  ///
  /// * [MDN Web Docs - BlobBuilder](https://developer.mozilla.org/en/DOM/BlobBuilder)
  /// * [MDN Web Docs - Blobs](https://developer.mozilla.org/en-US/docs/DOM/Blob)
  Blobbuilder,
  /// Blob URLs
  ///
  /// Method of creating URL handles to the specified File or Blob object.
  ///
  /// * [MDN Web Docs - createObjectURL](https://developer.mozilla.org/en/DOM/window.URL.createObjectURL)
  Bloburls,
  /// CSS3 Border images
  ///
  /// Method of using images for borders
  ///
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/border-image)
  /// * [MDN Web Docs - Border image](https://developer.mozilla.org//docs/Web/CSS/border-image)
  BorderImage,
  /// CSS3 Border-radius (rounded corners)
  ///
  /// Method of making the border corners round. Covers support for the shorthand `border-radius` as well as the long-hand properties (e.g. `border-top-left-radius`)
  ///
  /// * [Border-radius CSS Generator](https://border-radius.com)
  /// * [Detailed compliance table](https://muddledramblings.com/table-of-css3-border-radius-compliance)
  /// * [Polyfill which includes border-radius](http://css3pie.com/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/border-radius)
  /// * [MDN Web Docs - CSS border-radius](https://developer.mozilla.org/en/docs/Web/CSS/border-radius)
  BorderRadius,
  /// BroadcastChannel
  ///
  /// BroadcastChannel allows scripts from the same origin but other browsing contexts (windows, workers) to send each other messages.
  ///
  /// * [MDN Web Docs - Broadcast Channel](https://developer.mozilla.org/en-US/docs/Web/API/BroadcastChannel)
  /// * [Shim - Broadcast Channel based on Localstorage, Indexeddb or Sockets](https://github.com/pubkey/broadcast-channel)
  Broadcastchannel,
  /// Brotli Accept-Encoding/Content-Encoding
  ///
  /// More effective lossless compression algorithm than gzip and deflate.
  ///
  /// * [Introducing Brotli](https://opensource.googleblog.com/2015/09/introducing-brotli-new-compression.html)
  /// * [Blink's intent to ship](https://groups.google.com/a/chromium.org/forum/m/#!msg/blink-dev/JufzX024oy0/WEOGbN43AwAJ)
  /// * [Official code repository](https://github.com/google/brotli)
  /// * [WebKit Bug 154859: Add support for format brotli for HTTP compression](https://bugs.webkit.org/show_bug.cgi?id=154859)
  Brotli,
  /// calc() as CSS unit value
  ///
  /// Method of allowing calculated values for length units, i.e. `width: calc(100% - 3em)`
  ///
  /// * [Mozilla Hacks article](https://hacks.mozilla.org/2010/06/css3-calc/)
  /// * [MDN Web Docs - calc](https://developer.mozilla.org/en/docs/Web/CSS/calc)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/functions/calc)
  Calc,
  /// Canvas (basic support)
  ///
  /// Method of generating fast, dynamic graphics using JavaScript.
  ///
  /// * [Tutorial by Mozilla](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial)
  /// * [Animation kit](http://glimr.rubyforge.org/cake/canvas.html)
  /// * [Another tutorial](https://diveintohtml5.info/canvas.html)
  /// * [Implementation for Internet Explorer](https://github.com/arv/ExplorerCanvas)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/graphics.js#canvas)
  /// * [Canvas Tutorial & Cheat Sheet](https://skilled.co/html-canvas/)
  /// * [MDN Web Docs - Canvas API](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API)
  Canvas,
  /// Canvas blend modes
  ///
  /// Method of defining the effect resulting from overlaying two layers on a Canvas element.
  ///
  /// * [Blog post](https://blogs.adobe.com/webplatform/2013/01/28/blending-features-in-canvas/)
  CanvasBlending,
  /// Text API for Canvas
  ///
  /// Method of displaying text on Canvas elements
  ///
  /// * [Examples by Mozilla](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Drawing_text)
  /// * [Support library](https://code.google.com/archive/p/canvas-text/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/graphics.js#canvas-text)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/canvas/CanvasRenderingContext2D/fillText)
  CanvasText,
  /// ch (character) unit
  ///
  /// Unit representing the width of the character "0" in the current font, of particular use in combination with monospace fonts.
  ///
  /// * [Blog post on using ch units](https://johndjameson.com/posts/making-sense-of-ch-units)
  /// * [What is the CSS ‘ch’ Unit?](https://meyerweb.com/eric/thoughts/2018/06/28/what-is-the-css-ch-unit/)
  ChUnit,
  /// ChaCha20-Poly1305 cipher suites for TLS
  ///
  /// A set of cipher suites used in Transport Layer Security (TLS) protocol, using ChaCha20 for symmetric encryption and Poly1305 for authentication.
  ///
  /// * [Chrome article](https://security.googleblog.com/2014/04/speeding-up-and-strengthening-https.html)
  /// * [SSL/TLS Capabilities of Your Browser by Qualys SSL Labs](https://www.ssllabs.com/ssltest/viewMyClient.html)
  Chacha20Poly1305,
  /// Channel messaging
  ///
  /// Method for having two-way communication between browsing contexts (using MessageChannel)
  ///
  /// * [An Introduction to HTML5 web messaging](https://dev.opera.com/articles/view/window-postmessage-messagechannel/#channel)
  /// * [MDN Web Docs - Channel Messaging API](https://developer.mozilla.org/en-US/docs/Web/API/Channel_Messaging_API)
  ChannelMessaging,
  /// ChildNode.remove()
  ///
  /// DOM node method to remove the node itself from the document.
  ///
  /// * [MDN Web Docs - ChildNode.remove](https://developer.mozilla.org/en-US/docs/Web/API/ChildNode/remove)
  ChildnodeRemove,
  /// classList (DOMTokenList)
  ///
  /// Method of easily manipulating classes on elements, using the `DOMTokenList` object.
  ///
  /// * [Mozilla Hacks article](https://hacks.mozilla.org/2010/01/classlist-in-firefox-3-6/)
  /// * [Polyfill script](https://github.com/eligrey/classList.js)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/Element/classList)
  /// * [SitePoint article](https://www.sitepoint.com/exploring-classlist-api/)
  /// * [Demo using classList](https://www.audero.it/demo/classlist-api-demo.html)
  /// * [MDN Web Docs - Element.classList](https://developer.mozilla.org/en-US/docs/Web/API/Element.classList)
  Classlist,
  /// Client Hints: DPR, Width, Viewport-Width
  ///
  /// DPR, Width, and Viewport-Width hints enable proactive content negotiation between client and server, enabling automated delivery of optimized assets - e.g. auto-negotiating image DPR resolution.
  ///
  /// * [Automating resource selection with Client Hints](https://developers.google.com/web/updates/2015/09/automating-resource-selection-with-client-hints)
  /// * [Mozilla Bug 935216 - Implement Client-Hints HTTP header](https://bugzilla.mozilla.org/show_bug.cgi?id=935216)
  /// * [WebKit Bug 145380 - Add Content-DPR header support](https://bugs.webkit.org/show_bug.cgi?id=145380)
  ClientHintsDprWidthViewport,
  /// Synchronous Clipboard API
  ///
  /// API to provide copy, cut and paste events as well as provide access to the OS clipboard.
  ///
  /// * [MDN Web Docs - ClipboardEvent](https://developer.mozilla.org/en-US/docs/Web/API/ClipboardEvent)
  /// * [Guide on cross-platform clipboard access](https://www.lucidchart.com/techblog/2014/12/02/definitive-guide-copying-pasting-javascript/)
  Clipboard,
  /// COLR/CPAL(v0) Font Formats
  ///
  /// The COLR table adds support for multi-colored glyphs in a manner that integrates with the rasterizers of existing text engines. COLRv0 only supports pure colors, does not support gradients, transformations and various blending modes.
  ///
  /// * [Where can I use color fonts](https://www.colorfonts.wtf/#w-node-85d8080e63a6-0134536f)
  /// * [DirectWrite's color fonts document](https://docs.microsoft.com/en-us/windows/win32/directwrite/color-fonts)
  /// * [A tool to check whether the browser supports OpenType color formats](https://pixelambacht.nl/chromacheck/)
  /// * [A variable color font](https://www.harbortype.com/fonts/rocher-color/)
  /// * [Make COLR/CPAL format fonts in Glyphs app](https://glyphsapp.com/learn/creating-a-microsoft-color-font)
  Colr,
  /// COLR/CPAL(v1) Font Formats
  ///
  /// COLRv1 is an improved version of COLRv0, this is also part of the OpenType specification. COLRv1 supports additional graphic capabilities. In addition to solid colors, gradient fills can be used, as well as more complex fills using other graphic operations, including affine transformations and various blending modes.
  ///
  /// * [COLRv1 Color Gradient Vector Fonts in Chrome 98](https://developer.chrome.com/blog/colrv1-fonts/)
  /// * [COLRv1 version of Noto Emoji](https://github.com/googlefonts/color-fonts)
  /// * [A tool to check whether the browser supports OpenType color formats](https://pixelambacht.nl/chromacheck/)
  /// * [Where can I use color fonts](https://www.colorfonts.wtf/#w-node-85d8080e63a6-0134536f)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1740525)
  /// * [WebKit position](https://lists.webkit.org/pipermail/webkit-dev/2021-March/031765.html)
  /// * [First Batch of Color Fonts Arrives on Google Fonts](https://material.io/blog/color-fonts-are-here)
  ColrV1,
  /// Node.compareDocumentPosition()
  ///
  /// Compares the relative position of two nodes to each other in the DOM tree.
  ///
  /// * [MDN Web Docs - Node.compareDocumentPosition](https://developer.mozilla.org/en-US/docs/Web/API/Node/compareDocumentPosition)
  Comparedocumentposition,
  /// Basic console logging functions
  ///
  /// Method of outputting data to the browser's console, intended for development purposes.
  ///
  /// * [MDN Web Docs - Console](https://developer.mozilla.org/en-US/docs/Web/API/Console)
  /// * [Chrome console reference](https://developer.chrome.com/devtools/docs/console-api)
  /// * [Edge/Internet Explorer console reference](https://msdn.microsoft.com/en-us/library/hh772169)
  ConsoleBasic,
  /// console.time and console.timeEnd
  ///
  /// Functions for measuring performance
  ///
  /// * [MDN Web Docs - Console.time](https://developer.mozilla.org/en-US/docs/Web/API/Console/time)
  ConsoleTime,
  /// const
  ///
  /// Declares a constant with block level scope
  ///
  /// * [Variables and Constants in ES6](https://generatedcontent.org/post/54444832868/variables-and-constants-in-es6)
  /// * [MDN Web Docs - const](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/const)
  Const,
  /// Constraint Validation API
  ///
  /// API for better control over form field validation. Includes support for `checkValidity()`, `setCustomValidity()`, `reportValidity()` and validation states.
  ///
  /// * [MDN article on constraint validation](https://developer.mozilla.org/en-US/docs/Web/Guide/HTML/HTML5/Constraint_validation)
  /// * [`reportValidity()` ponyfill](https://github.com/jelmerdemaat/report-validity)
  ConstraintValidation,
  /// contenteditable attribute (basic support)
  ///
  /// Method of making any HTML element editable.
  ///
  /// * [WHATWG blog post](https://blog.whatwg.org/the-road-to-html-5-contenteditable)
  /// * [Blog post on usage problems](https://accessgarage.wordpress.com/2009/05/08/how-to-hack-your-app-to-make-contenteditable-work/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/attributes/contentEditable)
  /// * [MDN Web Docs - contentEditable attribute](https://developer.mozilla.org/en/docs/Web/API/HTMLElement/contentEditable)
  Contenteditable,
  /// Content Security Policy 1.0
  ///
  /// Mitigate cross-site scripting attacks by only allowing certain sources of script, style, and other resources.
  ///
  /// * [HTML5Rocks article](https://www.html5rocks.com/en/tutorials/security/content-security-policy/)
  /// * [CSP Examples & Quick Reference](https://content-security-policy.com/)
  /// * [MDN Web Docs - Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
  Contentsecuritypolicy,
  /// Content Security Policy Level 2
  ///
  /// Mitigate cross-site scripting attacks by only allowing certain sources of script, style, and other resources. CSP 2 adds hash-source, nonce-source, and five new directives
  ///
  /// * [HTML5Rocks article](https://www.html5rocks.com/en/tutorials/security/content-security-policy/)
  /// * [MDN Web Docs - Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
  Contentsecuritypolicy2,
  /// Cookie Store API
  ///
  /// An API for reading and modifying cookies. Compared to the existing `document.cookie` method, the API provides a much more modern interface, which can also be used in service workers.
  ///
  /// * [Article on using the Cookie Store API](https://developers.google.com/web/updates/2018/09/asynchronous-access-to-http-cookies)
  /// * [Specification explainer](https://wicg.github.io/cookie-store/explainer.html)
  /// * [Firefox position: defer](https://mozilla.github.io/standards-positions/#cookie-store)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1475599)
  /// * [WebKit position](https://github.com/WebKit/standards-positions/issues/36)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=258504)
  CookieStoreApi,
  /// Cross-Origin Resource Sharing
  ///
  /// Method of performing XMLHttpRequests across domains
  ///
  /// * [Mozilla Hacks blog post](https://hacks.mozilla.org/2009/07/cross-site-xmlhttprequest-with-cors/)
  /// * [Alternative implementation by IE8](https://msdn.microsoft.com/en-us/library/cc288060(VS.85).aspx)
  /// * [DOM access using CORS](https://dev.opera.com/articles/view/dom-access-control-using-cross-origin-resource-sharing/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-cors-xhr)
  /// * [MDN Web Docs - Access control CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Access_control_CORS)
  Cors,
  /// createImageBitmap
  ///
  /// Create image bitmap with support for resizing and adjusting quality
  ///
  /// * [self.createImageBitmap() - Web APIs | MDN](https://developer.mozilla.org/en-US/docs/Web/API/WindowOrWorkerGlobalScope/createImageBitmap)
  Createimagebitmap,
  /// Credential Management API
  ///
  /// API that provides a programmatic interface to the browser's credential manager. In short, an origin can request a user's credentials to sign them in, or can ask the browser to save credentials on the user's behalf. Both of these requests are user-mediated.
  ///
  /// * [Tutorial by Google](https://developers.google.com/web/updates/2016/04/credential-management-api)
  /// * [MDN Web Docs - Credential Management API](https://developer.mozilla.org/en-US/docs/Web/API/Credential_Management_API)
  /// * [Live Demo](https://credential-management-sample.appspot.com/)
  /// * [Sample Code](https://github.com/GoogleChrome/credential-management-sample)
  /// * [Spec discussion](https://github.com/w3c/webappsec-credential-management)
  CredentialManagement,
  /// Web Cryptography
  ///
  /// JavaScript API for performing basic cryptographic operations in web applications
  ///
  /// * [The History and Status of Web Crypto API](https://www.slideshare.net/Channy/the-history-and-status-of-web-crypto-api)
  /// * [Microsoft Research JavaScript Cryptography Library](https://github.com/microsoft/MSR-JavaScript-Crypto)
  /// * [Cross-browser cryptography library](https://bitwiseshiftleft.github.io/sjcl/)
  /// * [Polyfill by Netflix with partial support](https://github.com/Netflix/NfWebCrypto)
  /// * [PKI.js - another crypto library for Public Key Infrastructure applications](https://github.com/GlobalSign/PKI.js)
  /// * [Test suite for various algorithms/methods](https://diafygi.github.io/webcrypto-examples/)
  /// * [Web Cryptography API shim for IE11 and Safari - set of bugfixes and workarounds of prefixed api implementations](https://github.com/vibornoff/webcrypto-shim)
  /// * [MDN Web Docs - Web Crypto API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Crypto_API)
  Cryptography,
  /// CSS all property
  ///
  /// A shorthand property for resetting all CSS properties except for `direction` and `unicode-bidi`.
  ///
  /// * [MDN Web Docs - CSS all](https://developer.mozilla.org/en-US/docs/Web/CSS/all)
  /// * [Resetting styles using `all: unset`](https://mcc.id.au/blog/2013/10/all-unset)
  /// * [WebKit bug 116966: [css3-cascade] Add support for `all` shorthand property](https://bugs.webkit.org/show_bug.cgi?id=116966)
  CssAll,
  /// CSS Anchor Positioning
  ///
  /// Allows placing elements anywhere on the page relative to an "anchor element", without regard to the layout of other elements besides their containing block
  ///
  /// * [Blog post on usage](https://developer.chrome.com/blog/tether-elements-to-each-other-with-css-anchor-positioning/)
  /// * [Polyfill](https://github.com/oddbird/css-anchor-positioning)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1838746)
  /// * [Blog post](https://12daysofweb.dev/2023/anchor-positioning/)
  /// * [WebKit position on Anchor Positioning](https://github.com/WebKit/standards-positions/issues/167)
  CssAnchorPositioning,
  /// CSS Animation
  ///
  /// Complex method of animating certain properties of an element
  ///
  /// * [Blog post on usage](https://robertnyman.com/2010/05/06/css3-animations/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/animations)
  CssAnimation,
  /// CSS :any-link selector
  ///
  /// The `:any-link` CSS pseudo-class matches all elements that match `:link` or `:visited`
  ///
  /// * [MDN Web Docs - CSS :any-link](https://developer.mozilla.org/en-US/docs/Web/CSS/:any-link)
  CssAnyLink,
  /// CSS Appearance
  ///
  /// The `appearance` property defines how elements (particularly form controls) appear by default. By setting the value to `none` the default appearance can be entirely redefined using other CSS properties.
  ///
  /// * [CSS Tricks article](https://css-tricks.com/almanac/properties/a/appearance/)
  /// * [Safari implementation bug for unprefixed `appearance`](https://bugs.webkit.org/show_bug.cgi?id=143842)
  CssAppearance,
  /// CSS Counter Styles
  ///
  /// The @counter-style CSS at-rule allows custom counter styles to be defined. A @counter-style rule defines how to convert a counter value into a string representation.
  ///
  /// * [MDN Web Docs - CSS counter style](https://developer.mozilla.org/en-US/docs/Web/CSS/@counter-style)
  /// * [CSS @counter-style Demo](https://mdn.github.io/css-examples/counter-style-demo/)
  /// * [WebKit bug on support for @counter-style](https://bugs.webkit.org/show_bug.cgi?id=167645)
  CssAtCounterStyle,
  /// CSS Backdrop Filter
  ///
  /// Method of applying filter effects (like blur, grayscale or hue) to content/elements below the target element.
  ///
  /// * [Blog post](https://product.voxmedia.com/til/2015/2/17/8053347/css-ios-transparency-with-webkit-backdrop-filter)
  /// * [MDN Web Docs - CSS backdrop filter](https://developer.mozilla.org/en-US/docs/Web/CSS/backdrop-filter)
  /// * [WebKit bug to unprefix `-webkit-backdrop-filter`](https://bugs.webkit.org/show_bug.cgi?id=224899)
  CssBackdropFilter,
  /// CSS background-position edge offsets
  ///
  /// Allows CSS background images to be positioned relative to the specified edge using the 3 to 4 value syntax. For example: `background-position: right 5px bottom 5px;` for positioning 5px from the bottom-right corner.
  ///
  /// * [MDN Web Docs - background-position](https://developer.mozilla.org/en-US/docs/Web/CSS/background-position)
  /// * [Basic information](https://briantree.se/quick-tip-06-use-four-value-syntax-properly-position-background-images/)
  CssBackgroundOffsets,
  /// CSS background-blend-mode
  ///
  /// Allows blending between CSS background images, gradients, and colors.
  ///
  /// * [codepen example](https://codepen.io/bennettfeely/pen/rxoAc)
  /// * [Blog post](https://medium.com/web-design-technique/6b51bf53743a)
  /// * [Demo](https://bennettfeely.com/gradients/)
  CssBackgroundblendmode,
  /// CSS box-decoration-break
  ///
  /// Controls whether the box's margins, borders, padding, and other decorations wrap the broken edges of the box fragments (when the box is split by a break (page/column/region/line).
  ///
  /// * [MDN Web Docs - CSS box-decoration-break](https://developer.mozilla.org/en-US/docs/Web/CSS/box-decoration-break)
  /// * [Demo of effect on box border](https://jsbin.com/xojoro/edit?css,output)
  /// * [Chromium bug to unprefix `-webkit-box-decoration-break`](https://bugs.chromium.org/p/chromium/issues/detail?id=682224)
  CssBoxdecorationbreak,
  /// CSS3 Box-shadow
  ///
  /// Method of displaying an inner or outer shadow effect to elements
  ///
  /// * [MDN Web Docs - box-shadow](https://developer.mozilla.org/En/CSS/-moz-box-shadow)
  /// * [Live editor](https://westciv.com/tools/boxshadows/index.html)
  /// * [Demo of various effects](http://tests.themasta.com/blogstuff/boxshadowdemo.html)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/box-shadow)
  CssBoxshadow,
  /// CSS Canvas Drawings
  ///
  /// Method of using HTML5 Canvas as a background image. Not currently part of any specification.
  ///
  /// * [WebKit blog post](https://webkit.org/blog/176/css-canvas-drawing/)
  CssCanvas,
  /// CSS caret-color
  ///
  /// The `caret-color` property allows the color to be set of the caret (blinking text insertion pointer) in an editable text area.
  ///
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=166572)
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/CSS/caret-color)
  CssCaretColor,
  /// CSS Cascade Layers
  ///
  /// The `@layer` at-rule allows authors to explicitly layer their styles in the cascade, before specificity and order of appearance are considered.
  ///
  /// * [The Future of CSS: Cascade Layers (CSS @layer)](https://www.bram.us/2021/09/15/the-future-of-css-cascade-layers-css-at-layer/)
  /// * [Chromium support bug](https://crbug.com/1095765)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1699215)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=220779)
  /// * [Collection of demos](https://codepen.io/collection/BNjmma)
  CssCascadeLayers,
  /// Scoped Styles: the @scope rule
  ///
  /// Allows CSS rules to be scoped to part of the document, with upper and lower limits described by selectors.
  ///
  /// * [Explainer](https://css.oddbird.net/scope/explainer/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1830512)
  /// * [WebKit position: support](https://github.com/WebKit/standards-positions/issues/13)
  /// * [An introduction to @scope in CSS](https://fullystacked.net/posts/scope-in-css/)
  CssCascadeScope,
  /// Case-insensitive CSS attribute selectors
  ///
  /// Including an `i` before the `]` in a CSS attribute selector causes the attribute value to be matched in an ASCII-case-insensitive manner. For example, `[b="xyz" i]` would match both `<a b="xyz">` and `<a b="XYZ">`.
  ///
  /// * [MDN Web Docs - CSS case-insensitive](https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors#case-insensitive)
  /// * [JS Bin testcase](https://jsbin.com/zutuna/edit?html,css,output)
  CssCaseInsensitive,
  /// CSS clip-path property (for HTML)
  ///
  /// Method of defining the visible region of an HTML element using SVG or a shape definition.
  ///
  /// * [CSS Tricks article](https://css-tricks.com/almanac/properties/c/clip/)
  /// * [Codepen Example Clipping an Image with a Polygon](https://codepen.io/dubrod/details/myNNyW/)
  /// * [Visual test cases](https://lab.iamvdo.me/css-svg-masks)
  /// * [Chromium bug for shapes in external SVGs](https://bugs.chromium.org/p/chromium/issues/detail?id=109212)
  /// * [WebKit bug for shapes in external SVGs](https://bugs.webkit.org/show_bug.cgi?id=104442)
  CssClipPath,
  /// CSS print-color-adjust
  ///
  /// The `print-color-adjust` (or `-webkit-print-color-adjust` as prefixed in WebKit/Blink browsers) property is a CSS extension that can be used to force printing of background colors and images.
  ///
  /// * [MDN web docs - print-color-adjust](https://developer.mozilla.org/en-US/docs/Web/CSS/print-color-adjust)
  /// * [Edge issue with print-color-adjust](https://web.archive.org/web/20190624214232/https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/12399195/)
  /// * [Chromium bug with print-color-adjust property](https://bugs.chromium.org/p/chromium/issues/detail?id=131054)
  CssColorAdjust,
  /// CSS color() function
  ///
  /// The CSS `color()` function allows the browser to display colors in any color space, such as the P3 color space which can display colors outside of the default sRGB color space.
  ///
  /// * [Chromium implementation bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1068610)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1128204)
  /// * [WebKit article on using color() with the P3 color space](https://webkit.org/blog/10042/wide-gamut-color-in-css-with-display-p3/)
  /// * [Color generator that uses color() with the P3 color space](https://p3colorpicker.cool/)
  CssColorFunction,
  /// CSS Conical Gradients
  ///
  /// Method of defining a conical or repeating conical color gradient as a CSS image.
  ///
  /// * [Client-side polyfill](https://leaverou.github.io/conic-gradient/)
  /// * [Server-side polyfill (PostCSS)](https://github.com/jonathantneal/postcss-conic-gradient)
  /// * [Mozilla bug #1175958: Implement conic gradients from CSS Image Values Level 4](https://bugzilla.mozilla.org/show_bug.cgi?id=1175958)
  /// * [MDN Web Docs - conic-gradient()](https://developer.mozilla.org/docs/Web/CSS/conic-gradient)
  CssConicGradients,
  /// CSS Container Queries (Size)
  ///
  /// Size queries in Container Queries provide a way to query the size of a container, and conditionally apply CSS to the content of that container.
  ///
  /// * [Container Queries: a Quick Start Guide](https://www.oddbird.net/2021/04/05/containerqueries/)
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Container_Queries)
  /// * [Chromium support bug](https://crbug.com/1145970)
  /// * [Collection of demos](https://codepen.io/collection/XQrgJo)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=229659)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1744221)
  /// * [Container Query Polyfill](https://github.com/GoogleChromeLabs/container-query-polyfill)
  CssContainerQueries,
  /// CSS Container Style Queries
  ///
  /// Style queries in Container Queries provide a way to query the current styling of a container, and conditionally apply additional CSS to the contents of that container.
  ///
  /// * [Getting Started with Style Queries](https://developer.chrome.com/blog/style-queries/)
  /// * [Style Queries](https://una.im/style-queries/)
  /// * [Container Queries: Style Queries](https://www.bram.us/2022/10/14/container-queries-style-queries/)
  /// * [CSS Style Queries](https://ishadeed.com/article/css-container-style-queries/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1795622)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=246605)
  CssContainerQueriesStyle,
  /// CSS Container Query Units
  ///
  /// Container Query Units specify a length relative to the dimensions of a query container. The units include: `cqw`, `cqh`, `cqi`, `cqb`, `cqmin`, and `cqmax`.
  ///
  /// * [Blog post: CSS Container Query Units](https://ishadeed.com/article/container-query-units/)
  /// * [CSS Tricks: Container Units Should Be Pretty Handy](https://css-tricks.com/container-units-should-be-pretty-handy/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1744231)
  CssContainerQueryUnits,
  /// CSS Containment
  ///
  /// The CSS `contain` property lets developers limit the scope of the browser's styles, layout and paint work for faster and more efficient rendering.
  ///
  /// * [Google Developers article](https://developers.google.com/web/updates/2016/06/css-containment)
  CssContainment,
  /// CSS content-visibility
  ///
  /// Provides control over when elements are rendered, so rendering can be skipped for elements not yet in the user's viewport.
  ///
  /// * [content-visibility: the new CSS property that boosts your rendering performance](https://web.dev/content-visibility/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1660384)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=236238)
  CssContentVisibility,
  /// CSS Counters
  ///
  /// Method of controlling number values in generated content, using the `counter-reset` and `counter-increment` properties.
  ///
  /// * [Tutorial and information](https://onwebdev.blogspot.com/2012/02/css-counters-tutorial.html)
  /// * [MDN Web Docs - CSS Counters](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Counter_Styles/Using_CSS_counters)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/counter-reset)
  CssCounters,
  /// Crisp edges/pixelated images
  ///
  /// Scales images with an algorithm that preserves edges and contrast, without smoothing colors or introducing blur. This is intended for images such as pixel art. Official values that accomplish this for the `image-rendering` property are `crisp-edges` and `pixelated`.
  ///
  /// * [MDN Web Docs - CSS Image rendering](https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering)
  /// * [HTML5Rocks article](https://developer.chrome.com/blog/pixelated/)
  /// * [Firefox bug #856337: Implement image-rendering: pixelated](https://bugzilla.mozilla.org/show_bug.cgi?id=856337)
  /// * [Chrome bug #317991: Implement image-rendering:crisp-edges](https://bugs.chromium.org/p/chromium/issues/detail?id=317991)
  CssCrispEdges,
  /// CSS Cross-Fade Function
  ///
  /// Image function to create a "crossfade" between images. This allows one image to transition (fade) into another based on a percentage value.
  ///
  /// * [Firefox bug #546052: Implement cross-fade()](https://bugzilla.mozilla.org/show_bug.cgi?id=546052)
  /// * [MDN Web Docs - CSS cross-fade()](https://developer.mozilla.org/en-US/docs/Web/CSS/cross-fade())
  /// * [Chromium bug to unprefix `-webkit-cross-fade()`](https://bugs.chromium.org/p/chromium/issues/detail?id=614906)
  CssCrossFade,
  /// :default CSS pseudo-class
  ///
  /// The `:default` pseudo-class matches checkboxes and radio buttons which are checked by default, `<option>`s with the `selected` attribute, and the default submit button (if any) of a form.
  ///
  /// * [HTML specification for `:default`](https://html.spec.whatwg.org/multipage/scripting.html#selector-default)
  /// * [MDN Web Docs - CSS :default](https://developer.mozilla.org/en-US/docs/Web/CSS/:default)
  /// * [WebKit bug 156230 - `:default` CSS pseudo-class should match checkboxes+radios with a `checked` attribute](https://bugs.webkit.org/show_bug.cgi?id=156230)
  /// * [JS Bin testcase](https://jsbin.com/hiyada/edit?html,css,output)
  CssDefaultPseudo,
  /// Explicit descendant combinator >>
  ///
  /// An explicit, non-whitespace spelling of the descendant combinator. `A >> B` is equivalent to `A B`.
  ///
  /// * [MDN Web Docs - Descendant selectors](https://developer.mozilla.org/en-US/docs/Web/CSS/Descendant_selectors)
  /// * [JS Bin testcase](https://jsbin.com/qipekof/edit?html,css,output)
  /// * [Chrome issue #446050: Implement Descendant Combinator ">>"](https://bugs.chromium.org/p/chromium/issues/detail?id=446050)
  /// * [Mozilla bug #1266283 - Implement CSS4 descendant combinator `>>`](https://bugzilla.mozilla.org/show_bug.cgi?id=1266283)
  CssDescendantGtgt,
  /// CSS Device Adaptation
  ///
  /// Method of overriding the size of viewport in web page using the `@viewport` rule, replacing Apple's own popular `<meta>` viewport implementation. Includes the `extend-to-zoom` width value.
  ///
  /// * [Introduction to meta viewport and @viewport in Opera Mobile](https://dev.opera.com/articles/view/an-introduction-to-meta-viewport-and-viewport/)
  /// * [Device adaptation in Internet Explorer 10](https://docs.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/dev-guides/hh708740(v=vs.85))
  /// * [Chrome tracking bug](https://code.google.com/p/chromium/issues/detail?id=155477)
  /// * [WebKit tracking bug](https://bugs.webkit.org/show_bug.cgi?id=95959)
  /// * [Mozilla tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=747754)
  CssDeviceadaptation,
  /// :dir() CSS pseudo-class
  ///
  /// Matches elements based on their directionality. `:dir(ltr)` matches elements which are Left-to-Right. `:dir(rtl)` matches elements which are Right-to-Left.
  ///
  /// * [HTML specification for `:dir()`](https://html.spec.whatwg.org/multipage/scripting.html#selector-ltr)
  /// * [MDN Web Docs - CSS :dir](https://developer.mozilla.org/en-US/docs/Web/CSS/:dir)
  /// * [Chrome issue #576815: CSS4 pseudo-class :dir()](https://bugs.chromium.org/p/chromium/issues/detail?id=576815)
  /// * [WebKit bug #64861: Need support for :dir() pseudo-class](https://bugs.webkit.org/show_bug.cgi?id=64861)
  /// * [JS Bin testcase](https://jsbin.com/celuye/edit?html,css,output)
  CssDirPseudo,
  /// CSS display: contents
  ///
  /// `display: contents` causes an element's children to appear as if they were direct children of the element's parent, ignoring the element itself. This can be useful when a wrapper element should be ignored when using CSS grid or similar layout techniques.
  ///
  /// * [Vanishing boxes with display contents](https://rachelandrew.co.uk/archives/2016/01/29/vanishing-boxes-with-display-contents/)
  CssDisplayContents,
  /// CSS element() function
  ///
  /// This function renders a live image generated from an arbitrary HTML element
  ///
  /// * [MDN Web Docs - CSS element](https://developer.mozilla.org/en-US/docs/Web/CSS/element)
  CssElementFunction,
  /// CSS Environment Variables env()
  ///
  /// Usage of environment variables like `safe-area-inset-top`.
  ///
  /// * [JSFiddle test case](https://jsfiddle.net/mrd3h90w/)
  /// * [The env() CSS Function - MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/CSS/env)
  /// * [Designing Websites for iPhone X - WebKit Blog](https://webkit.org/blog/7929/designing-websites-for-iphone-x/)
  CssEnvFunction,
  /// CSS Exclusions Level 1
  ///
  /// Exclusions defines how inline content flows around elements. It extends the content wrapping ability of floats to any block-level element.
  ///
  /// * [CSS Exclusions](https://msdn.microsoft.com/en-us/library/ie/hh673558(v=vs.85).aspx)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=674804)
  /// * [WebKit tracking bug](https://bugs.webkit.org/show_bug.cgi?id=57311)
  /// * [Chromium tracking bug](https://crbug.com/700838)
  CssExclusions,
  /// CSS Feature Queries
  ///
  /// CSS Feature Queries allow authors to condition rules based on whether particular property declarations are supported in CSS using the @supports at rule.
  ///
  /// * [MDN Web Docs - CSS @supports](https://developer.mozilla.org/en-US/docs/Web/CSS/@supports)
  /// * [@supports in Firefox](https://mcc.id.au/blog/2012/08/supports)
  /// * [Test case](https://dabblet.com/gist/3895764)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/atrules/supports)
  CssFeaturequeries,
  /// CSS filter() function
  ///
  /// This function filters a CSS input image with a set of filter functions (like blur, grayscale or hue)
  ///
  /// * [Blog post](https://iamvdo.me/en/blog/advanced-css-filters#filter)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1191043)
  /// * [Chromium support bug](https://crbug.com/541698)
  CssFilterFunction,
  /// CSS Filter Effects
  ///
  /// Method of applying filter effects using the `filter` property to elements, matching filters available in SVG. Filter functions include blur, brightness, contrast, drop-shadow, grayscale, hue-rotate, invert, opacity, sepia and saturate.
  ///
  /// * [Demo](https://fhtr.org/css-filters/)
  /// * [HTML5Rocks article](https://www.html5rocks.com/en/tutorials/filters/understanding-css/)
  /// * [Filter editor](https://web.archive.org/web/20160219005748/https://dl.dropboxusercontent.com/u/3260327/angular/CSS3ImageManipulation.html)
  /// * [Filter Playground](https://web.archive.org/web/20160310041612/http://bennettfeely.com/filters/)
  CssFilters,
  /// ::first-letter CSS pseudo-element selector
  ///
  /// CSS pseudo-element that allows styling only the first "letter" of text within an element. Useful for implementing initial caps or drop caps styling.
  ///
  /// * [MDN Web Docs - :first-letter](https://developer.mozilla.org/en-US/docs/Web/CSS/::first-letter)
  CssFirstLetter,
  /// CSS first-line pseudo-element
  ///
  /// Allows styling specifically for the first line of text using the `::first-line` pseudo-element. Note that only a limited set of properties can be applied.
  ///
  /// * [MDN Web Docs - ::first-line](https://developer.mozilla.org/en-US/docs/Web/CSS/::first-line)
  /// * [CSS tricks article](https://css-tricks.com/almanac/selectors/f/first-line/)
  CssFirstLine,
  /// CSS position:fixed
  ///
  /// Method of keeping an element in a fixed location regardless of scroll position
  ///
  /// * [Article on mobile support](https://bradfrost.com/blog/post/fixed-position/)
  /// * [position: fixed on MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/position#fixed)
  CssFixed,
  /// :focus-visible CSS pseudo-class
  ///
  /// The `:focus-visible` pseudo-class applies while an element matches the `:focus` pseudo-class, and the UA determines via heuristics that the focus should be specially indicated on the element (typically via a “focus ring”).
  ///
  /// * [Prototype for `:focus-visible`](https://github.com/WICG/focus-visible)
  /// * [Chrome does not support CSS Selectors 4 :focus-visible](https://bugs.chromium.org/p/chromium/issues/detail?id=817199)
  /// * [Blink: Intent to implement :focus-visible pseudo class.](https://groups.google.com/a/chromium.org/forum/#!topic/blink-dev/-wN72ESFsyo)
  /// * [Mozilla Developer Network (MDN) documentation - :-moz-focusring](https://developer.mozilla.org/en-US/docs/Web/CSS/:-moz-focusring)
  /// * [Bugzilla: Add :focus-visible (former :focus-ring)](https://bugzilla.mozilla.org/show_bug.cgi?id=1437901)
  /// * [Bugzilla: implement :focus-visible pseudo-class (rename/alias :-moz-focusring)](https://bugzilla.mozilla.org/show_bug.cgi?id=1445482)
  /// * [WebKit bug #185859: [selectors] Support for Focus-Indicated Pseudo-class: `:focus-visible`](https://bugs.webkit.org/show_bug.cgi?id=185859)
  CssFocusVisible,
  /// :focus-within CSS pseudo-class
  ///
  /// The `:focus-within` pseudo-class matches elements that either themselves match `:focus` or that have descendants which match `:focus`.
  ///
  /// * [The Future Generation of CSS Selectors: Level 4: Generalized Input Focus Pseudo-class](https://www.sitepoint.com/future-generation-css-selectors-level-4/#generalized-input-focus-pseudo-class-focus-within)
  /// * [ally.style.focusWithin Polyfill, part of ally.js](https://allyjs.io/api/style/focus-within.html)
  /// * [WebKit bug #140144: Add support for CSS4 `:focus-within` pseudo](https://bugs.webkit.org/show_bug.cgi?id=140144)
  /// * [Chromium issue #617371: Implement `:focus-within` pseudo-class from Selectors Level 4](https://bugs.chromium.org/p/chromium/issues/detail?id=617371)
  /// * [Mozilla bug #1176997: Add support for pseudo class `:focus-within`](https://bugzilla.mozilla.org/show_bug.cgi?id=1176997)
  /// * [MDN Web Docs - :focus-within](https://developer.mozilla.org/en-US/docs/Web/CSS/:focus-within)
  /// * [JS Bin testcase](https://jsbin.com/qevoqa/edit?html,css,output)
  CssFocusWithin,
  /// CSS font-palette
  ///
  /// The font-palette CSS property allows selecting a palette from a color font. In combination with the `@font-palette-values` at-rule, custom palettes can be defined.
  ///
  /// * [Demo](https://yisibl.github.io/color-font-palette/)
  /// * [Explainer](https://github.com/drott/csswg-drafts/blob/paletteExplainer/css-fonts-4/palette-explainer.md)
  /// * [Safari 15.4 Beta Release Notes](https://webkit.org/blog/12445/new-webkit-features-in-safari-15-4/#typography)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1461588)
  CssFontPalette,
  /// CSS font-display
  ///
  /// `@font-face` descriptor `font-display` that allows control over how a downloadable font renders before it is fully loaded.
  ///
  /// * [Google Developers article](https://developers.google.com/web/updates/2016/02/font-display)
  /// * [MDN Web Docs - font-display](https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-display)
  /// * [CSS tricks article](https://css-tricks.com/font-display-masses/)
  CssFontRenderingControls,
  /// CSS font-stretch
  ///
  /// If a font has multiple types of variations based on the width of characters, the `font-stretch` property allows the appropriate one to be selected. The property in itself does not cause the browser to stretch to a font.
  ///
  /// * [MDN Web Docs - font-stretch](https://developer.mozilla.org/en-US/docs/Web/CSS/font-stretch)
  /// * [CSS Tricks article](https://css-tricks.com/almanac/properties/f/font-stretch/)
  CssFontStretch,
  /// CSS Generated content for pseudo-elements
  ///
  /// Method of displaying text or images before or after the given element's contents using the ::before and ::after pseudo-elements. All browsers with support also support the `attr()` notation in the `content` property.
  ///
  /// * [Guide on usage](https://www.westciv.com/style_master/academy/css_tutorial/advanced/generated_content.html)
  /// * [Dev.Opera article](https://dev.opera.com/articles/view/css-generated-content-techniques/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/generated_and_replaced_content)
  CssGencontent,
  /// CSS Gradients
  ///
  /// Method of defining a linear or radial color gradient as a CSS image.
  ///
  /// * [Cross-browser editor](https://www.colorzilla.com/gradient-editor/)
  /// * [Tool to emulate support in IE](http://css3pie.com/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/functions/linear-gradient)
  CssGradients,
  /// CSS Grid Layout (level 1)
  ///
  /// Method of using a grid concept to lay out content, providing a mechanism for authors to divide available space for layout into columns and rows using a set of predictable sizing behaviors. Includes support for all `grid-*` properties and the `fr` unit.
  ///
  /// * [Polyfill based on old spec](https://github.com/codler/Grid-Layout-Polyfill)
  /// * [Polyfill based on new spec](https://github.com/FremyCompany/css-grid-polyfill/)
  /// * [WebKit Blog post](https://webkit.org/blog/7434/css-grid-layout-a-new-layout-module-for-the-web/)
  /// * [Css Grid By Example: Everything you need to learn CSS Grid Layout](https://gridbyexample.com/)
  /// * [Mozilla: Introduction to CSS Grid Layout](https://mozilladevelopers.github.io/playground/css-grid)
  CssGrid,
  /// CSS hanging-punctuation
  ///
  /// Allows some punctuation characters from start (or the end) of text elements to be placed "outside" of the box in order to preserve the reading flow.
  ///
  /// * [CSS tricks article](https://css-tricks.com/almanac/properties/h/hanging-punctuation/)
  /// * [Firefox bug #1253615](https://bugzilla.mozilla.org/show_bug.cgi?id=1253615)
  /// * [Chrome bug #41491716](https://issues.chromium.org/issues/41491716)
  CssHangingPunctuation,
  /// :has() CSS relational pseudo-class
  ///
  /// Select elements containing specific content. For example, `a:has(img)` selects all `<a>` elements that contain an `<img>` child.
  ///
  /// * [MDN Web Docs - :has](https://developer.mozilla.org/en-US/docs/Web/CSS/:has)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=418039)
  /// * [Chrome bug to track implementation](https://bugs.chromium.org/p/chromium/issues/detail?id=669058)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=227702)
  /// * [Using :has() as a CSS Parent Selector and much more](https://webkit.org/blog/13096/css-has-pseudo-class/)
  CssHas,
  /// CSS Hyphenation
  ///
  /// Method of controlling when words at the end of lines should be hyphenated using the "hyphens" property.
  ///
  /// * [MDN Web Docs - CSS hyphens](https://developer.mozilla.org/en-US/docs/Web/CSS/hyphens)
  /// * [Blog post](https://clagnut.com/blog/2394)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/hyphens)
  /// * [Chromium bug for implementing hyphenation](https://bugs.chromium.org/p/chromium/issues/detail?id=652964)
  /// * [WebKit bug to unprefix `-webkit-hyphens`](https://bugs.webkit.org/show_bug.cgi?id=193002)
  CssHyphens,
  /// CSS3 image-orientation
  ///
  /// CSS property used generally to fix the intended orientation of an image. This can be done using 90 degree increments or based on the image's EXIF data using the "from-image" value.
  ///
  /// * [MDN Web Docs - CSS image-orientation](https://developer.mozilla.org/en-US/docs/Web/CSS/image-orientation)
  /// * [Blog post](http://sethfowler.org/blog/2013/09/13/new-in-firefox-26-css-image-orientation/)
  /// * [Demo (Chinese)](https://jsbin.com/EXUTolo/4)
  /// * [Chromium bug #158753: Support for the CSS image-orientation CSS property](https://bugs.chromium.org/p/chromium/issues/detail?id=158753)
  CssImageOrientation,
  /// CSS image-set
  ///
  /// Method of letting the browser pick the most appropriate CSS image from a given set.
  ///
  /// * [Web Platform Tests](https://wpt.fyi/results/css/css-images/image-set/image-set-parsing.html)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=image-set)
  /// * [Chromium bug to update and unprefix image-set()](https://bugs.chromium.org/p/chromium/issues/detail?id=630597)
  /// * [WebKit bug to support type()](https://bugs.webkit.org/show_bug.cgi?id=225185)
  CssImageSet,
  /// :in-range and :out-of-range CSS pseudo-classes
  ///
  /// If a temporal or number `<input>` has `max` and/or `min` attributes, then `:in-range` matches when the value is within the specified range and `:out-of-range` matches when the value is outside the specified range. If there are no range constraints, then neither pseudo-class matches.
  ///
  /// * [MDN Web Docs - CSS :out-of-range](https://developer.mozilla.org/en-US/docs/Web/CSS/:out-of-range)
  /// * [WHATWG HTML specification for `:in-range` and `:out-of-range`](https://html.spec.whatwg.org/multipage/scripting.html#selector-in-range)
  CssInOutOfRange,
  /// :indeterminate CSS pseudo-class
  ///
  /// The `:indeterminate` pseudo-class matches indeterminate checkboxes, indeterminate `<progress>` bars, and radio buttons with no checked button in their radio button group.
  ///
  /// * [HTML specification for `:indeterminate`](https://html.spec.whatwg.org/multipage/scripting.html#selector-indeterminate)
  /// * [MDN Web Docs - CSS :indeterminate](https://developer.mozilla.org/en-US/docs/Web/CSS/:indeterminate)
  /// * [EdgeHTML issue 7124038 - `:indeterminate` pseudo-class doesn't match radio buttons](https://web.archive.org/web/20190624214229/https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/7124038/)
  /// * [Mozilla Bug 885359 - Radio groups without a selected radio button should have `:indeterminate` applying](https://bugzilla.mozilla.org/show_bug.cgi?id=885359)
  /// * [WebKit Bug 156270 - `:indeterminate` pseudo-class should match radios whose group has no checked radio](https://bugs.webkit.org/show_bug.cgi?id=156270)
  /// * [JS Bin testcase](https://jsbin.com/zumoqu/edit?html,css,js,output)
  CssIndeterminatePseudo,
  /// CSS Initial Letter
  ///
  /// Method of creating an enlarged cap, including a drop or raised cap, in a robust way.
  ///
  /// * [Firefox Implementation Ticket](https://bugzilla.mozilla.org/show_bug.cgi?id=1273019)
  /// * [MDN Web Docs - CSS initial-letter](https://developer.mozilla.org/en-US/docs/Web/CSS/initial-letter)
  /// * [Blog post on Envato Tuts+, "Better CSS Drop Caps With initial-letter"](https://webdesign.tutsplus.com/tutorials/better-css-drop-caps-with-initial-letter--cms-26350)
  /// * [Demos at Jen Simmons Labs](https://labs.jensimmons.com/#initialletter)
  /// * [WebKit bug to unprefix -webkit-initial-letter](https://bugs.webkit.org/show_bug.cgi?id=229090)
  CssInitialLetter,
  /// CSS initial value
  ///
  /// A CSS value that will apply a property's initial value as defined in the CSS specification that defines the property
  ///
  /// * [MDN Web Docs - CSS initial](https://developer.mozilla.org/en-US/docs/Web/CSS/initial)
  /// * [CSS Tricks article](https://css-tricks.com/getting-acquainted-with-initial/)
  CssInitialValue,
  /// LCH and Lab color values
  ///
  /// The `lch()` and `lab()` color functions are based on the CIE LAB color space, representing colors in a way that closely matches human perception and provides access to a wider spectrum of colors than offered by the usual RGB color space.
  ///
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1026287)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1352757)
  /// * [LCH colors in CSS: what, why, and how?](https://lea.verou.me/2020/04/lch-colors-in-css-what-why-and-how/)
  /// * [LCH color picker](https://css.land/lch/)
  /// * [MDN article on lch()](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/lch())
  /// * [MDN article on lab()](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/lab())
  CssLchLab,
  /// letter-spacing CSS property
  ///
  /// Controls spacing between characters of text (i.e. "tracking" in typographical terms). Not to be confused with kerning.
  ///
  /// * [MDN Web Docs - CSS letter-spacing](https://developer.mozilla.org/en-US/docs/Web/CSS/letter-spacing)
  CssLetterSpacing,
  /// CSS line-clamp
  ///
  /// CSS property that will contain text to a given amount of lines when used in combination with `display: -webkit-box`. It will end with ellipsis when `text-overflow: ellipsis` is included.
  ///
  /// * [CSS Tricks article](https://css-tricks.com/line-clampin/)
  CssLineClamp,
  /// CSS Logical Properties
  ///
  /// Logical properties and values provide control of layout through logical, rather than physical, direction and dimension mappings. These properties are `writing-mode` relative equivalents of their corresponding physical properties.
  ///
  /// * [MDN Web Docs - CSS -moz-margin-start](https://developer.mozilla.org/en-US/docs/Web/CSS/-moz-margin-start)
  /// * [MDN Web Docs - CSS -moz-padding-start](https://developer.mozilla.org/en-US/docs/Web/CSS/-moz-padding-start)
  /// * [MDN - Basic concepts of Logical Properties and Values](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Logical_Properties/Basic_concepts)
  CssLogicalProps,
  /// CSS ::marker pseudo-element
  ///
  /// The `::marker` pseudo-element allows list item markers to be styled or have their content value customized.
  ///
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=457718)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=205202)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=141477)
  /// * [MDN Web Docs - CSS ::marker](https://developer.mozilla.org/en-US/docs/Web/CSS/::marker)
  /// * [CSS-Tricks article](https://css-tricks.com/almanac/selectors/m/marker/)
  CssMarkerPseudo,
  /// CSS Masks
  ///
  /// Method of displaying part of an element, using a selected image as a mask
  ///
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/mask)
  /// * [HTML5 Rocks article](https://www.html5rocks.com/en/tutorials/masking/adobe/)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1224422)
  /// * [Visual test cases](https://lab.iamvdo.me/css-svg-masks)
  /// * [Detailed blog post (via The Internet Archive)](https://web.archive.org/web/20160505054016/http://thenittygritty.co/css-masking)
  CssMasks,
  /// :is() CSS pseudo-class
  ///
  /// The `:is()` (formerly `:matches()`, formerly `:any()`) pseudo-class checks whether the element at its position in the outer selector matches any of the selectors in its selector list. It's useful syntactic sugar that allows you to avoid writing out all the combinations manually as separate selectors. The effect is similar to nesting in Sass and most other CSS preprocessors.
  ///
  /// * [WebKit blog post about adding `:matches()` and other Selectors Level 4 features](https://webkit.org/blog/3615/css-selectors-inside-selectors-discover-matches-not-and-nth-child/)
  /// * [Chrome support bug for :is()](https://bugs.chromium.org/p/chromium/issues/detail?id=568705)
  /// * [MDN Web Docs - CSS :is()](https://developer.mozilla.org/en-US/docs/Web/CSS/:is)
  /// * [Codepen - Modern tests](https://codepen.io/atjn/full/MWKErBe)
  /// * [JS Bin - Legacy tests](https://output.jsbin.com/lehina)
  CssMatchesPseudo,
  /// CSS math functions min(), max() and clamp()
  ///
  /// More advanced mathematical expressions in addition to `calc()`
  ///
  /// * [Test case on JSFiddle](https://jsfiddle.net/as9t4jek/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=css-min-max)
  /// * [Chrome support bug](https://crbug.com/825895)
  /// * [MDN Web Docs article for min()](https://developer.mozilla.org/en-US/docs/Web/CSS/min)
  /// * [MDN Web Docs article for max()](https://developer.mozilla.org/en-US/docs/Web/CSS/max)
  /// * [MDN Web Docs article for clamp()](https://developer.mozilla.org/en-US/docs/Web/CSS/clamp)
  /// * [Getting Started With CSS Math Functions Level 4](https://webdesign.tutsplus.com/tutorials/mathematical-expressions-calc-min-and-max--cms-29735)
  /// * [Introduction to CSS Math Functions](https://stackdiary.com/css-math-functions/)
  CssMathFunctions,
  /// Media Queries: interaction media features
  ///
  /// Allows a media query to be set based on the presence and accuracy of the user's pointing device, and whether they have the ability to hover over elements on the page. This includes the `pointer`, `any-pointer`, `hover`, and `any-hover` media features.
  ///
  /// * [Potential use cases for script, hover and pointer CSS Level 4 Media Features](https://jordanm.co.uk/2013/11/11/potential-use-cases-for-script-hover-and-pointer.html)
  /// * [Interaction Media Features and their potential (for incorrect assumptions)](https://dev.opera.com/articles/media-features/)
  /// * [Polyfill for the `hover` media feature](https://github.com/twbs/mq4-hover-shim)
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=398943)
  CssMediaInteraction,
  /// Media Queries: Range Syntax
  ///
  /// Syntax improvements to make media queries using features that have a "range" type (like width or height) less verbose. Can be used with ordinary mathematical comparison operators: `>`, `<`, `>=`, or `<=`.
  ///
  /// For example: `@media (100px <= width <= 1900px)` is the equivalent of `@media (min-width: 100px) and (max-width: 1900px)`
  ///
  /// * [Syntax improvements in Level 4](https://developer.mozilla.org/en-US/docs/Web/CSS/Media_Queries/Using_media_queries)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1422225#c55)
  /// * [PostCSS Polyfill](https://github.com/postcss/postcss-media-minmax)
  /// * [WebKit bug](https://bugs.webkit.org/show_bug.cgi?id=180234)
  /// * [Media Queries Level 4: Media Query Range Contexts (Media Query Ranges)](https://www.bram.us/2021/10/26/media-queries-level-4-media-query-range-contexts/)
  /// * [New syntax for range media queries in Chrome 104](https://developer.chrome.com/blog/media-query-range-syntax/)
  CssMediaRangeSyntax,
  /// Media Queries: resolution feature
  ///
  /// Allows a media query to be set based on the device pixels used per CSS unit. While the standard uses `min`/`max-resolution` for this, some browsers support the older non-standard `device-pixel-ratio` media query.
  ///
  /// * [How to unprefix -webkit-device-pixel-ratio](https://www.w3.org/blog/CSS/2012/06/14/unprefix-webkit-device-pixel-ratio/)
  /// * [WebKit Bug 78087: Implement the 'resolution' media query](https://bugs.webkit.org/show_bug.cgi?id=78087)
  /// * [WHATWG Compatibility Standard: -webkit-device-pixel-ratio](https://compat.spec.whatwg.org/#css-media-queries-webkit-device-pixel-ratio)
  /// * [MDN Web Docs - CSS @media resolution](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/resolution)
  /// * [CSS Values and Units Module Level 4 add the `x` unit as an alias for `dppx`.](https://drafts.csswg.org/css-values/#dppx)
  /// * [Chrome support 'x' as a resolution unit.](https://chromestatus.com/feature/5150549246738432)
  /// * [Firefox support 'x' as a resolution unit.](https://bugzilla.mozilla.org/show_bug.cgi?id=1460655)
  CssMediaResolution,
  /// CSS3 Media Queries
  ///
  /// Method of applying styles based on media information. Includes things like page and device dimensions
  ///
  /// * [IE demo page with information](https://testdrive-archive.azurewebsites.net/HTML5/85CSS3_MediaQueries/)
  /// * [Media Queries tutorial](https://webdesignerwall.com/tutorials/responsive-design-with-css3-media-queries)
  /// * [Polyfill for IE](https://github.com/scottjehl/Respond)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/atrules/media)
  /// * [Practical Guide to Media Queries](https://stackdiary.com/css-media-queries/)
  CssMediaqueries,
  /// Blending of HTML/SVG elements
  ///
  /// Allows blending between arbitrary SVG and HTML elements
  ///
  /// * [codepen example](https://codepen.io/bennettfeely/pen/csjzd)
  /// * [Blog post](https://css-tricks.com/basics-css-blend-modes/)
  CssMixblendmode,
  /// CSS Motion Path
  ///
  /// Allows elements to be animated along SVG paths or shapes via the `offset-path` property. Originally defined as the `motion-path` property.
  ///
  /// * [Blog post](https://codepen.io/danwilson/post/css-motion-paths)
  /// * [MDN Web Docs - CSS motion-path](https://developer.mozilla.org/en-US/docs/Web/CSS/motion-path)
  /// * [Demo](https://googlechrome.github.io/samples/css-motion-path/index.html)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1186329)
  CssMotionPaths,
  /// CSS namespaces
  ///
  /// Using the `@namespace` at-rule, elements of other namespaces (e.g. SVG) can be targeted using the pipe (`|`) selector.
  ///
  /// * [MDN Web Docs - CSS @namespace](https://developer.mozilla.org/en-US/docs/Web/CSS/@namespace)
  CssNamespaces,
  /// CSS Nesting
  ///
  /// CSS nesting provides the ability to nest one style rule inside another, with the selector of the child rule relative to the selector of the parent rule. Similar behavior previously required a CSS pre-processor.
  ///
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1095675)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1648037)
  /// * [Safari support bug](https://bugs.webkit.org/show_bug.cgi?id=223497)
  /// * [Blog post: CSS Nesting, specificity and you](https://kilianvalkhof.com/2021/css-html/css-nesting-specificity-and-you/)
  CssNesting,
  /// selector list argument of :not()
  ///
  /// Selectors Level 3 only allowed `:not()` pseudo-class to accept a single simple selector, which the element must not match any of. Thus, `:not(a, .b, [c])` or `:not(a.b[c])` did not work. Selectors Level 4 allows `:not()` to accept a list of selectors. Thus, `:not(a):not(.b):not([c])` can instead be written as `:not(a, .b, [c])` and `:not(a.b[c])` works as intended.
  ///
  /// * [MDN Web Docs - CSS :not](https://developer.mozilla.org/en-US/docs/Web/CSS/:not)
  /// * [Chrome feature request issue](https://bugs.chromium.org/p/chromium/issues/detail?id=580628)
  /// * [Firefox feature request bug](https://bugzilla.mozilla.org/show_bug.cgi?id=933562)
  CssNotSelList,
  /// selector list argument of :nth-child and :nth-last-child CSS pseudo-classes
  ///
  /// The newest versions of `:nth-child()` and `:nth-last-child()` accept an optional `of S` clause which filters the children to only those which match the selector list `S`. For example, `:nth-child(1 of .foo)` selects the first child among the children that have the `foo` class (ignoring any non-`foo` children which precede that child). Similar to `:nth-of-type`, but for arbitrary selectors instead of only type selectors.
  ///
  /// * [Mozilla Bug 854148 - Support for :nth-child(An+B of sel), :nth-last-child(An+B of sel) pseudo-classes](https://bugzilla.mozilla.org/show_bug.cgi?id=854148)
  /// * [Chromium Issue 304163: Implement :nth-child(an+b of S) and :nth-last-child(an+b of S) pseudo-classes](https://bugs.chromium.org/p/chromium/issues/detail?id=304163)
  /// * [MS Edge Platform Status: Under Consideration](https://web.archive.org/web/20190401105447if_/https://developer.microsoft.com/en-us/microsoft-edge/platform/status/cssselectorslevel4/)
  CssNthChildOf,
  /// CSS3 Opacity
  ///
  /// Method of setting the transparency level of an element
  ///
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/opacity)
  CssOpacity,
  /// :optional CSS pseudo-class
  ///
  /// The `:optional` pseudo-class matches form inputs (`<input>`, `<textarea>`, `<select>`) which are not `:required`.
  ///
  /// * [HTML specification for `:optional`](https://html.spec.whatwg.org/multipage/scripting.html#selector-optional)
  /// * [MDN Web Docs - CSS :optional](https://developer.mozilla.org/en-US/docs/Web/CSS/:optional)
  /// * [JS Bin testcase](https://jsbin.com/fihudu/edit?html,css,output)
  CssOptionalPseudo,
  /// CSS overflow property
  ///
  /// Originally a single property for controlling overflowing content in both horizontal & vertical directions, the `overflow` property is now a shorthand for `overflow-x` & `overflow-y`. The latest version of the specification also introduces the `clip` value that blocks programmatic scrolling.
  ///
  /// * [CSS overflow on MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow)
  /// * [WebKit bug on support for two values syntax](https://bugs.webkit.org/show_bug.cgi?id=184691)
  /// * [Edge bug on support for two values syntax](https://web.archive.org/web/20190401105108/https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/16993428/)
  /// * [WebKit bug on support for clip value](https://bugs.webkit.org/show_bug.cgi?id=198230)
  CssOverflow,
  /// CSS overflow-anchor (Scroll Anchoring)
  ///
  /// Changes in DOM elements above the visible region of a scrolling box can result in the page moving while the user is in the middle of consuming the content.
  /// By default, the value of  `overflow-anchor` is `auto`, it can mitigate this jarring user experience by keeping track of the position of an anchor node and adjusting the scroll offset accordingly
  ///
  /// * [Explainer](https://github.com/WICG/ScrollAnchoring/blob/master/explainer.md)
  /// * [Google developers article](https://developers.google.com/web/updates/2016/04/scroll-anchoring)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=171099)
  CssOverflowAnchor,
  /// CSS overflow: overlay
  ///
  /// The `overlay` value of the `overflow` CSS property is a non-standard value to make scrollbars appear on top of content rather than take up space. This value is deprecated and related functionality being standardized as [the `scrollbar-gutter` property](mdn-css_properties_scrollbar-gutter).
  ///
  /// * [MDN article on overflow values](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow#values)
  /// * [WebKit change to make "overflow: overlay" a synonym for "overflow: auto"](https://trac.webkit.org/changeset/236341/webkit)
  CssOverflowOverlay,
  /// CSS overscroll-behavior
  ///
  /// CSS property to control the behavior when the scroll position of a scroll container reaches the edge of the scrollport.
  ///
  /// * [Demo](https://ebidel.github.io/demos/chatbox.html)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=951793#c11)
  /// * [Google Developers blog post on overscroll-behavior](https://developers.google.com/web/updates/2017/11/overscroll-behavior)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=176454)
  /// * [CSS overscroll-behavior on MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/overscroll-behavior)
  CssOverscrollBehavior,
  /// CSS page-break properties
  ///
  /// Properties to control the way elements are broken across (printed) pages.
  ///
  /// * [CSS Tricks article](https://css-tricks.com/almanac/properties/p/page-break/)
  /// * [Latest fragmentation specification (includes column & region breaks)](https://drafts.csswg.org/css-break-3/#break-between)
  CssPageBreak,
  /// CSS Paged Media (@page)
  ///
  /// CSS at-rule (`@page`) to define page-specific rules when printing web pages, such as margin per page and page dimensions.
  ///
  /// * [CSS Paged media article](https://www.tutorialspoint.com/css/css_paged_media.htm)
  /// * [MDN Web Docs - CSS @page](https://developer.mozilla.org/en-US/docs/Web/CSS/@page)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=85062)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=286443)
  CssPagedMedia,
  /// CSS Painting API
  ///
  /// Allows programmatic generation of images used by CSS
  ///
  /// * [Google CSS Paint API Introduction](https://developers.google.com/web/updates/2018/01/paintapi)
  /// * [Is Houdini Ready Yet?](https://ishoudinireadyyet.com/)
  CssPaintApi,
  /// ::placeholder CSS pseudo-element
  ///
  /// The ::placeholder pseudo-element represents placeholder text in an input field: text that represents the input and provides a hint to the user on how to fill out the form. For example, a date-input field might have the placeholder text `YYYY-MM-DD` to clarify that numeric dates are to be entered in year-month-day order.
  ///
  /// * [CSS-Tricks article with all prefixes](https://css-tricks.com/snippets/css/style-placeholder-text/)
  /// * [CSSWG discussion](https://wiki.csswg.org/ideas/placeholder-styling)
  /// * [MDN Web Docs - CSS ::-moz-placeholder](https://developer.mozilla.org/en-US/docs/Web/CSS/::-moz-placeholder)
  /// * [Mozilla Bug 1069012 - unprefix :placeholder-shown pseudo-class and ::placeholder pseudo-element](https://bugzilla.mozilla.org/show_bug.cgi?id=1069012)
  /// * [MDN web docs - ::placeholder](https://developer.mozilla.org/en-US/docs/Web/CSS/::placeholder)
  CssPlaceholder,
  /// :placeholder-shown CSS pseudo-class
  ///
  /// Input elements can sometimes show placeholder text as a hint to the user on what to type in. See, for example, the placeholder attribute in HTML5. The :placeholder-shown pseudo-class matches an input element that is showing such placeholder text.
  ///
  /// * [WebKit commit](https://trac.webkit.org/changeset/172826)
  /// * [Firefox bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1069015)
  CssPlaceholderShown,
  /// CSS :read-only and :read-write selectors
  ///
  /// :read-only and :read-write pseudo-classes to match elements which are considered user-alterable
  ///
  /// * [CSS Tricks article](https://css-tricks.com/almanac/selectors/r/read-write-read/)
  /// * [MDN :read-only](https://developer.mozilla.org/en-US/docs/Web/CSS/%3Aread-only)
  /// * [MDN Web Docs - CSS :read-write](https://developer.mozilla.org/en-US/docs/Web/CSS/:read-write)
  /// * [Selectors Level 4 § The Mutability Pseudo-classes: :read-only and :read-write](https://drafts.csswg.org/selectors-4/#rw-pseudos)
  /// * [Firefox feature request bug](https://bugzilla.mozilla.org/show_bug.cgi?id=312971)
  CssReadOnlyWrite,
  /// Rebeccapurple color
  ///
  /// The new color added in CSS Color Module Level 4
  ///
  /// * [Codepen blog post](https://codepen.io/trezy/post/honoring-a-great-man)
  CssRebeccapurple,
  /// CSS Reflections
  ///
  /// Method of displaying a reflection of an element
  ///
  /// * [WebKit blog post](https://webkit.org/blog/182/css-reflections/)
  CssReflections,
  /// CSS Regions
  ///
  /// Method of flowing content into multiple elements, allowing magazine-like layouts. While once supported in WebKit-based browsers and Internet Explorer, implementing the feature is no longer being pursued by any browser.
  ///
  /// * [Adobe demos and samples](https://web.archive.org/web/20121027050852/http://html.adobe.com:80/webstandards/cssregions/)
  /// * [IE10 developer guide info](https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/dev-guides/hh673537(v=vs.85))
  /// * [Firefox feature request bug](https://bugzilla.mozilla.org/show_bug.cgi?id=674802)
  /// * [Beginner's guide to CSS Regions](https://www.sitepoint.com/a-beginners-guide-css-regions/)
  /// * [Discussion on removal in Blink](https://groups.google.com/a/chromium.org/g/blink-dev/c/kTktlHPJn4Q/m/YrnfLxeMO7IJ)
  CssRegions,
  /// CSS Relative color syntax
  ///
  /// Relative color syntax in CSS allows a color to be defined relative to another color using the `from` keyword and optionally `calc()` for any of the color values.
  ///
  /// * [Dynamic Color Manipulation with CSS Relative Colors](https://blog.jim-nielsen.com/2021/css-relative-colors/)
  /// * [Chromium bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1274133)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1701488)
  /// * [Relative color syntax (blog post)](https://www.matuzo.at/blog/2023/100daysof-day92/)
  CssRelativeColors,
  /// CSS Repeating Gradients
  ///
  /// Method of defining a repeating linear or radial color gradient as a CSS image.
  ///
  /// * [MDN Web Docs - CSS repeating linear gradient](https://developer.mozilla.org/en/CSS/repeating-linear-gradient)
  CssRepeatingGradients,
  /// CSS resize property
  ///
  /// Method of allowing an element to be resized by the user, with options to limit to a given direction.
  ///
  /// * [CSS Tricks info](https://css-tricks.com/almanac/properties/r/resize/)
  /// * [On textarea resizing](https://davidwalsh.name/textarea-resize)
  /// * [CSS resize none on textarea is bad for UX](https://catalin.red/css-resize-none-is-bad-for-ux/)
  CssResize,
  /// CSS revert value
  ///
  /// A CSS keyword value that resets a property's value to the default specified by the browser in its UA stylesheet, as if the webpage had not included any CSS. For example, `display:revert` on a `<div>` would result in `display:block`. This is in contrast to the `initial` value, which is simply defined on a per-property basis, and for `display` would be `inline`.
  ///
  /// * [MDN Web Docs - CSS revert](https://developer.mozilla.org/en-US/docs/Web/CSS/revert)
  /// * [Firefox feature request bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1215878)
  /// * [Chrome feature request issue](https://code.google.com/p/chromium/issues/detail?id=579788)
  CssRevertValue,
  /// #rrggbbaa hex color notation
  ///
  /// The CSS Color Module Level 4 defines new 4 & 8 character hex notation for color to include the opacity level.
  ///
  /// * [JS Bin testcase](https://jsbin.com/ruyetahatu/edit?html,css,output)
  CssRrggbbaa,
  /// CSS Scroll-behavior
  ///
  /// Method of specifying the scrolling behavior for a scrolling box, when scrolling happens due to navigation or CSSOM scrolling APIs.
  ///
  /// * [MDN Web Docs - CSS scroll-behavior](https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-behavior)
  /// * [Chrome launch bug ](https://code.google.com/p/chromium/issues/detail?id=243871)
  /// * [Blog post with demo](https://hospodarets.com/native_smooth_scrolling)
  /// * [iOS / WebKit bug report](https://bugs.webkit.org/show_bug.cgi?id=188043)
  CssScrollBehavior,
  /// CSS scrollbar styling
  ///
  /// Methods of styling scrollbars' color and width.
  ///
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1460109)
  /// * [Stackoverflow article discussing cross-browser support](https://stackoverflow.com/questions/9251354/css-customized-scroll-bar-in-div/14150577#14150577)
  /// * [Tutorial for IE & WebKit/Blink browsers](http://codemug.com/html/custom-scrollbars-using-css/)
  /// * ["perfect-scrollbar" - Minimal custom scrollbar plugin](https://perfectscrollbar.com/)
  /// * [jQuery custom content scroller](https://manos.malihu.gr/jquery-custom-content-scroller/)
  /// * [Webkit blog post describing their non-standard support](https://webkit.org/blog/363/styling-scrollbars/)
  CssScrollbar,
  /// CSS 2.1 selectors
  ///
  /// Basic CSS selectors including: `*` (universal selector), `>` (child selector), `:first-child`, `:link`, `:visited`, `:active`, `:hover`, `:focus`, `:lang()`, `+` (adjacent sibling selector), `[attr]`, `[attr="val"]`, `[attr~="val"]`, `[attr|="bar"]`, `.foo` (class selector), `#foo` (id selector)
  ///
  /// * [Detailed support information](https://www.quirksmode.org/css/contents.html)
  /// * [Examples of advanced selectors](https://www.yourhtmlsource.com/stylesheets/advancedselectors.html)
  /// * [Selectivizr: Polyfill for IE6-8](http://selectivizr.com)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/selectors)
  CssSel2,
  /// CSS3 selectors
  ///
  /// Advanced element selection using selectors including: `[foo^="bar"]`, `[foo$="bar"]`, `[foo*="bar"]`, `:root`, `:nth-child()`,  `:nth-last-child()`, `:nth-of-type()`, `:nth-last-of-type()`, `:last-child`, `:first-of-type`, `:last-of-type`, `:only-child`, `:only-of-type`, `:empty`, `:target`, `:enabled`, `:disabled`, `:checked`, `:not()`, `~` (general sibling)
  ///
  /// * [Detailed support information](https://www.quirksmode.org/css/selectors/)
  /// * [Automated CSS3 selector test](http://www.css3.info/selectors-test/)
  /// * [Selectivizr: Polyfill for IE6-8](http://selectivizr.com)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/selectors)
  CssSel3,
  /// ::selection CSS pseudo-element
  ///
  /// The ::selection CSS pseudo-element applies rules to the portion of a document that has been highlighted (e.g., selected with the mouse or another pointing device) by the user.
  ///
  /// * [::selection test](https://quirksmode.org/css/selectors/selection.html)
  /// * [MDN web docs](https://developer.mozilla.org/en-US/docs/Web/CSS/::selection)
  CssSelection,
  /// CSS Shapes Level 1
  ///
  /// Allows geometric shapes to be set in CSS to define an area for text to flow around. Includes properties `shape-outside`, `shape-margin` and `shape-image-threshold`
  ///
  /// * [A List Apart article](https://alistapart.com/article/css-shapes-101/)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1040714)
  CssShapes,
  /// CSS Scroll Snap
  ///
  /// CSS technique that allows customizable scrolling experiences like pagination of carousels by setting defined snap positions.
  ///
  /// * [Blog post](https://generatedcontent.org/post/66817675443/setting-native-like-scrolling-offsets-in-css-with)
  /// * [MDN Web Docs - CSS Scroll snap points](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Scroll_Snap_Points)
  /// * [Polyfill - based on an [older version](https://www.w3.org/TR/2015/WD-css-snappoints-1-20150326/) of the spec](https://github.com/ckrack/scrollsnap-polyfill)
  /// * [Polyfill - based on the [current version](https://www.w3.org/TR/css-scroll-snap-1/) of the spec](https://www.npmjs.com/package/css-scroll-snap-polyfill)
  /// * [MDN Web Docs - CSS Scroll Snap](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Scroll_Snap)
  /// * [A CSS Snap Points based carousel (and lightweight polyfill) ](https://github.com/filamentgroup/snapper)
  CssSnappoints,
  /// CSS position:sticky
  ///
  /// Keeps elements positioned as "fixed" or "relative" depending on how it appears in the viewport. As a result the element is "stuck" when necessary while scrolling.
  ///
  /// * [HTML5Rocks](https://developers.google.com/web/updates/2012/08/Stick-your-landings-position-sticky-lands-in-WebKit)
  /// * [MDN Web Docs - CSS position](https://developer.mozilla.org/en-US/docs/Web/CSS/position)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/position)
  /// * [Polyfill](https://github.com/dollarshaveclub/stickybits)
  /// * [Another polyfill](https://github.com/wilddeer/stickyfill)
  /// * [geddski article: Examples and Gotchas](https://mastery.games/post/position-sticky/)
  CssSticky,
  /// CSS Subgrid
  ///
  /// Feature of the CSS Grid Layout Module Level 2 that allows a grid-item with its own grid to align in one or both dimensions with its parent grid.
  ///
  /// * [CSS Grid Level 2: Here Comes Subgrid](https://www.smashingmagazine.com/2018/07/css-grid-2/)
  /// * [ Why we need CSS subgrid ](https://dev.to/kenbellows/why-we-need-css-subgrid-53mh)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1240834)
  /// * [Chromium support bug](https://crbug.com/618969)
  /// * [Webkit support bug](https://bugs.webkit.org/show_bug.cgi?id=202115)
  CssSubgrid,
  /// CSS.supports() API
  ///
  /// The CSS.supports() static method returns a Boolean value indicating if the browser supports a given CSS feature, or not.
  ///
  /// * [MDN Web Docs - CSS supports()](https://developer.mozilla.org/en-US/docs/Web/API/CSS.supports)
  /// * [Demo (Chinese)](https://jsbin.com/rimevilotari/1/edit?html,output)
  /// * [Native CSS Feature Detection via the @supports Rule](https://dev.opera.com/articles/native-css-feature-detection/)
  /// * [CSS @supports](https://davidwalsh.name/css-supports)
  /// * [Article (Chinese)](https://blog.csdn.net/hfahe/article/details/8619480)
  CssSupportsApi,
  /// CSS Table display
  ///
  /// Method of displaying elements as tables, rows, and cells. Includes support for all `display: table-*` properties as well as `display: inline-table`
  ///
  /// * [Blog post on usage](https://www.onenaught.com/posts/201/use-css-displaytable-for-layout)
  CssTable,
  /// CSS3 text-align-last
  ///
  /// CSS property to describe how the last line of a block or a line right before a forced line break when `text-align` is `justify`.
  ///
  /// * [MDN Web Docs - CSS text-align-last](https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-last)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=146772)
  CssTextAlignLast,
  /// CSS text-box-trim & text-box-edge
  ///
  /// Provides the ability to remove the vertical space appearing above and below text glyphs, allowing more precise positioning and alignment.
  ///
  /// Previously specified as the `leading-trim` & `text-edge` properties.
  ///
  ///
  /// * [Document with examples of text-box-trim uses](https://github.com/jantimon/text-box-trim-examples)
  /// * [CSS Tricks article](https://css-tricks.com/leading-trim-the-future-of-digital-typesetting/)
  CssTextBoxTrim,
  /// CSS text-indent
  ///
  /// The `text-indent` property applies indentation to lines of inline content in a block.
  ///
  /// * [MDN Web Docs - CSS text-indent](https://developer.mozilla.org/en-US/docs/Web/CSS/text-indent)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=784648)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=112755)
  /// * [Article on using text-indent for image replacement](https://www.sitepoint.com/css-image-replacement-text-indent-negative-margins-and-more/)
  CssTextIndent,
  /// CSS text-justify
  ///
  /// CSS property to define how text should be justified when `text-align: justify` is set.
  ///
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=248894)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=99945)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=276079)
  CssTextJustify,
  /// CSS text-orientation
  ///
  /// The CSS `text-orientation` property specifies the orientation of text within a line. Current values only have an effect in vertical typographic modes (defined with the `writing-mode` property)
  ///
  /// * [CSS text-orientation on MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/text-orientation)
  CssTextOrientation,
  /// CSS text-wrap: balance
  ///
  /// Allows multiple lines of text to have their lines broken in such a way that each line is roughly the same width, often used to make headlines more readable and visually appealing.
  ///
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=249840)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1731541)
  /// * [Blog post](https://dev.to/hunzaboy/text-balance-in-css-is-coming-17e3)
  /// * [Polyfill](https://github.com/adobe/balance-text)
  CssTextWrapBalance,
  /// CSS3 Text-shadow
  ///
  /// Method of applying one or more shadow or blur effects to text
  ///
  /// * [Mozilla hacks article](https://hacks.mozilla.org/2009/06/text-shadow/)
  /// * [Live editor](https://testdrive-archive.azurewebsites.net/Graphics/hands-on-css3/hands-on_text-shadow.htm)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/text-shadow)
  CssTextshadow,
  /// CSS touch-action property
  ///
  /// touch-action is a CSS property that controls filtering of gesture events, providing developers with a declarative mechanism to selectively disable touch scrolling (in one or both axes) or double-tap-zooming.
  ///
  /// * [300ms tap delay, gone away](https://developer.chrome.com/blog/300ms-tap-delay-gone-away/)
  /// * [What Exactly Is..... The 300ms Click Delay](https://www.telerik.com/blogs/what-exactly-is.....-the-300ms-click-delay)
  /// * [MDN Web Docs - CSS touch-action](https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action)
  CssTouchAction,
  /// CSS3 Transitions
  ///
  /// Simple method of animating certain properties of an element, with ability to define property, duration, delay and timing function.
  ///
  /// * [Article on usage](https://www.webdesignerdepot.com/2010/01/css-transitions-101/)
  /// * [Examples on timing functions](https://www.the-art-of-web.com/css/timing-function/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/transition)
  CssTransitions,
  /// CSS unset value
  ///
  /// A CSS value that's the same as "inherit" if a property is inherited or "initial" if a property is not inherited.
  ///
  /// * [MDN Web Docs - CSS unset](https://developer.mozilla.org/en-US/docs/Web/CSS/unset)
  /// * [Resetting styles using `all: unset`](https://mcc.id.au/blog/2013/10/all-unset)
  /// * [WebKit bug 148614: Add support for the `unset` CSS property value](https://bugs.webkit.org/show_bug.cgi?id=148614)
  CssUnsetValue,
  /// CSS Variables (Custom Properties)
  ///
  /// Permits the declaration and usage of cascading variables in stylesheets.
  ///
  /// * [Mozilla hacks article (older syntax)](https://hacks.mozilla.org/2013/12/css-variables-in-firefox-nightly/)
  /// * [MDN Web Docs - Using CSS variables](https://developer.mozilla.org/en-US/docs/Web/CSS/Using_CSS_variables)
  /// * [Edge Dev Blog post](https://blogs.windows.com/msedgedev/2017/03/24/css-custom-properties/)
  /// * [Polyfill for IE11](https://github.com/nuxodin/ie11CustomProperties)
  CssVariables,
  /// CSS @when / @else conditional rules
  ///
  /// Syntax allowing CSS conditions (like media and support queries) to be written more simply, as well as making it possible to write mutually exclusive rules using `@else` statements.
  ///
  /// * [Blog post: Extending CSS when/else chains: A first look](https://blog.logrocket.com/extending-css-when-else-chains-first-look/)
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1282896)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1747727)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=234701)
  CssWhenElse,
  /// CSS widows & orphans
  ///
  /// CSS properties to control when lines break across pages or columns by defining the amount of lines that must be left before or after the break.
  ///
  /// * [CSS last-line: Controlling Widows & Orphans](https://thenewcode.com/946/CSS-last-line-Controlling-Widows-amp-Orphans)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=137367)
  /// * [codrops article on orphans](https://tympanus.net/codrops/css_reference/orphans/)
  /// * [codrops article on widows](https://tympanus.net/codrops/css_reference/widows/)
  CssWidowsOrphans,
  /// CSS writing-mode property
  ///
  /// Property to define whether lines of text are laid out horizontally or vertically and the direction in which blocks progress.
  ///
  /// * [MDN Web Docs - CSS writing-mode](https://developer.mozilla.org/en-US/docs/Web/CSS/writing-mode)
  /// * [Chrome Platform Status](https://www.chromestatus.com/feature/5707470202732544)
  CssWritingMode,
  /// CSS zoom
  ///
  /// Method of scaling content while also affecting layout.
  ///
  /// * [CSS Tricks](https://css-tricks.com/almanac/properties/z/zoom/)
  /// * [Safari Developer Library](https://developer.apple.com/library/safari/documentation/AppleApplications/Reference/SafariCSSRef/Articles/StandardCSSProperties.html#//apple_ref/doc/uid/TP30001266-SW1)
  /// * [Article explaining usage of zoom as the hack for fixing rendering bugs in IE6 and IE7.](https://web.archive.org/web/20160809134322/http://www.satzansatz.de/cssd/onhavinglayout.html)
  /// * [MDN Web Docs - CSS zoom](https://developer.mozilla.org/en-US/docs/Web/CSS/zoom)
  CssZoom,
  /// CSS3 attr() function for all properties
  ///
  /// While `attr()` is supported for effectively all browsers for the `content` property, CSS Values and Units Level 5 adds the ability to use `attr()` on **any** CSS property, and to use it for non-string values (e.g. numbers, colors).
  ///
  /// * [MDN Web Docs - CSS attr](https://developer.mozilla.org/en-US/docs/Web/CSS/attr)
  /// * [Mozilla Bug #435426: implement css3-values extensions to `attr()`](https://bugzilla.mozilla.org/show_bug.cgi?id=435426)
  /// * [Chromium issue #246571: Implement CSS3 attribute / attr references](https://bugs.chromium.org/p/chromium/issues/detail?id=246571)
  /// * [WebKit Bug #26609: Support CSS3 attr() function](https://bugs.webkit.org/show_bug.cgi?id=26609)
  Css3Attr,
  /// CSS3 Box-sizing
  ///
  /// Method of specifying whether or not an element's borders and padding should be included in size units
  ///
  /// * [MDN Web Docs - CSS box-sizing](https://developer.mozilla.org/En/CSS/Box-sizing)
  /// * [Blog post](https://www.456bereastreet.com/archive/201104/controlling_width_with_css3_box-sizing/)
  /// * [Polyfill for IE](https://github.com/Schepp/box-sizing-polyfill)
  /// * [CSS Tricks](https://css-tricks.com/box-sizing/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/box-sizing)
  Css3Boxsizing,
  /// CSS3 Colors
  ///
  /// Method of describing colors using Hue, Saturation and Lightness (hsl()) rather than just RGB, as well as allowing alpha-transparency with rgba() and hsla().
  ///
  /// * [Dev.Opera article](https://dev.opera.com/articles/view/color-in-opera-10-hsl-rgb-and-alpha-transparency/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/color#RGBA_Notation)
  Css3Colors,
  /// CSS3 Cursors (original values)
  ///
  /// CSS3 cursor values added in the 2004 spec, including none, context-menu, cell, vertical-text, alias, copy, no-drop, not-allowed, nesw-resize, nwse-resize, col-resize, row-resize and all-scroll.
  ///
  /// * [MDN Web Docs - CSS cursor](https://developer.mozilla.org/en-US/docs/Web/CSS/cursor)
  Css3Cursors,
  /// CSS grab & grabbing cursors
  ///
  /// Support for the `grab` & `grabbing` values for the `cursor` property. Used to indicate that something can be grabbed (dragged to be moved).
  ///
  /// * [MDN Web Docs - CSS cursor](https://developer.mozilla.org/en-US/docs/Web/CSS/cursor)
  Css3CursorsGrab,
  /// CSS3 Cursors: zoom-in & zoom-out
  ///
  /// Support for `zoom-in`, `zoom-out` values for the CSS3 `cursor` property.
  ///
  /// * [MDN Web Docs - CSS cursor](https://developer.mozilla.org/en-US/docs/Web/CSS/cursor)
  Css3CursorsNewer,
  /// CSS3 tab-size
  ///
  /// Method of customizing the width of the tab character. Only effective using 'white-space: pre', 'white-space: pre-wrap', and 'white-space: break-spaces'.
  ///
  /// * [MDN Web Docs - CSS tab-size](https://developer.mozilla.org/en-US/docs/Web/CSS/tab-size)
  /// * [Firefox bug to unprefix `-moz-tab-size`](https://bugzilla.mozilla.org/show_bug.cgi?id=737785)
  Css3Tabsize,
  /// CSS currentColor value
  ///
  /// A CSS value that will apply the existing `color` value to other properties like `background-color`, etc.
  ///
  /// * [MDN Web Docs - CSS currentColor](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value#currentColor_keyword)
  /// * [CSS Tricks article](https://css-tricks.com/currentcolor/)
  Currentcolor,
  /// Custom Elements (deprecated V0 spec)
  ///
  /// Original V0 version of the custom elements specification. See [Custom Elements V1](#feat=custom-elementsv1) for support for the latest version.
  ///
  /// * [Blog post on upgrading from V0 to V1](https://developer.chrome.com/blog/web-components-time-to-upgrade/)
  CustomElements,
  /// Custom Elements (V1)
  ///
  /// One of the key features of the Web Components system, custom elements allow new HTML tags to be defined.
  ///
  /// * [Google Developers - Custom elements v1: reusable web components](https://developers.google.com/web/fundamentals/primers/customelements/)
  /// * [customElements.define polyfill](https://github.com/webcomponents/polyfills/tree/master/packages/custom-elements)
  /// * [WebKit Blog: Introducing Custom Elements](https://webkit.org/blog/7027/introducing-custom-elements/)
  CustomElementsv1,
  /// CustomEvent
  ///
  /// A DOM event interface that can carry custom application-defined data.
  ///
  /// * [MDN Web Docs - CustomEvent](https://developer.mozilla.org/en-US/docs/Web/API/CustomEvent)
  /// * [Polyfill based on the MDN snippet](https://github.com/krambuhl/custom-event-polyfill)
  /// * [EventListener polyfill which includes a CustomEvent polyfill](https://github.com/jonathantneal/EventListener)
  Customevent,
  /// Datalist element
  ///
  /// Method of setting a list of options for a user to select in a text field, while leaving the ability to enter a custom value.
  ///
  /// * [Mozilla Hacks article](https://hacks.mozilla.org/2010/11/firefox-4-html5-forms/)
  /// * [HTML5 Library including datalist support](https://afarkas.github.io/webshim/demos/)
  /// * [MDN Web Docs - datalist](https://developer.mozilla.org/en/HTML/Element/datalist)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/datalist)
  /// * [Eiji Kitamura's options demos & tests](https://demo.agektmr.com/datalist/)
  /// * [Minimal Datalist polyfill w/tutorial](https://github.com/thgreasi/datalist-polyfill)
  /// * [Minimal and library dependency-free vanilla JavaScript polyfill](https://github.com/mfranzke/datalist-polyfill)
  Datalist,
  /// dataset & data-* attributes
  ///
  /// Method of applying and accessing custom data to elements.
  ///
  /// * [HTML5 Doctor article](https://html5doctor.com/html5-custom-data-attributes/)
  /// * [Demo using dataset](https://html5demos.com/dataset)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/dom.js#dom-dataset)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/attributes/data-*)
  /// * [MDN Web Docs - dataset](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement.dataset)
  /// * [MDN Guide - Using data-* attributes](https://developer.mozilla.org/en-US/docs/Web/Guide/HTML/Using_data_attributes)
  Dataset,
  /// Data URIs
  ///
  /// Method of embedding images and other files in webpages as a string of text, generally using base64 encoding.
  ///
  /// * [Information page](https://css-tricks.com/data-uris/)
  /// * [Wikipedia](https://en.wikipedia.org/wiki/data_URI_scheme)
  /// * [Data URL converter](https://www.websiteoptimization.com/speed/tweak/inline-images/)
  /// * [Information on security issues](https://klevjers.com/papers/phishing.pdf)
  Datauri,
  /// Date.prototype.toLocaleDateString
  ///
  /// Date method to generate a language sensitive representation of a given date, formatted based on a specified locale and options.
  ///
  /// * [MDN article on  Date​.prototype​.toLocale​Date​String()](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleDateString)
  DateTolocaledatestring,
  /// Declarative Shadow DOM
  ///
  /// Proposal to allow rendering elements with shadow dom (aka web components) using server-side rendering.
  ///
  /// * [Declarative Shadow DOM - web.dev article](https://web.dev/declarative-shadow-dom/)
  /// * [A ponyfill of the Declarative Shadow DOM API](https://www.npmjs.com/package/@webcomponents/template-shadowroot)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=249513)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1712140)
  DeclarativeShadowDom,
  /// Decorators
  ///
  /// ECMAScript Decorators are an in-progress proposal for extending JavaScript classes. Decorators use a special syntax, prefixed with an `@` symbol and placed immediately before the code being extended.
  ///
  /// * [JavaScript Decorators: What They Are and When to Use Them](https://www.sitepoint.com/javascript-decorators-what-they-are/)
  /// * [A minimal guide to JavaScript (ECMAScript) Decorators and Property Descriptor of the Object](https://medium.com/jspoint/a-minimal-guide-to-ecmascript-decorators-55b70338215e)
  /// * [Decorators in TypeScript](https://www.typescriptlang.org/docs/handbook/decorators.html)
  /// * [Babel plug-in for decorators](https://babeljs.io/docs/en/babel-plugin-proposal-decorators)
  /// * [Bug on Firefox support](https://bugzilla.mozilla.org/show_bug.cgi?id=1781212)
  Decorators,
  /// Details & Summary elements
  ///
  /// The <details> element generates a simple no-JavaScript widget to show/hide element contents, optionally by clicking on its child <summary> element.
  ///
  /// * [jQuery fallback script](https://mathiasbynens.be/notes/html5-details-jquery)
  /// * [Fallback script](https://gist.github.com/370590)
  /// * [HTML5 Doctor article](https://html5doctor.com/summary-figcaption-element/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-details)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/details)
  /// * [Bug on Firefox support](https://bugzilla.mozilla.org/show_bug.cgi?id=591737)
  /// * [Details Element Polyfill](https://github.com/javan/details-element-polyfill)
  Details,
  /// DeviceOrientation & DeviceMotion events
  ///
  /// API for detecting orientation and motion events from the device running the browser.
  ///
  /// * [HTML5 Rocks tutorial](https://www.html5rocks.com/en/tutorials/device/orientation/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-orientation)
  /// * [DeviceOrientation implementation prototype for IE10](http://html5labs.interoperabilitybridges.com/prototypes/device-orientation-events/device-orientation-events/info)
  /// * [Demo](https://audero.it/demo/device-orientation-api-demo.html)
  Deviceorientation,
  /// Window.devicePixelRatio
  ///
  /// Read-only property that returns the ratio of the (vertical) size of one physical pixel on the current display device to the size of one CSS pixel.
  ///
  /// * [MDN Web Docs - devicePixelRatio](https://developer.mozilla.org/en-US/docs/Web/API/Window/devicePixelRatio)
  Devicepixelratio,
  /// Dialog element
  ///
  /// Method of easily creating custom dialog boxes to display to the user with modal or non-modal options. Also includes a `::backdrop` pseudo-element for behind the element.
  ///
  /// * [Polyfill](https://github.com/GoogleChrome/dialog-polyfill)
  Dialog,
  /// EventTarget.dispatchEvent
  ///
  /// Method to programmatically trigger a DOM event.
  ///
  /// * [MDN Web Docs - dispatchEvent](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/dispatchEvent)
  /// * [Financial Times IE8 polyfill](https://github.com/Financial-Times/polyfill-service/blob/master/polyfills/Event/polyfill-ie8.js)
  /// * [WebReflection ie8 polyfill](https://github.com/WebReflection/ie8)
  Dispatchevent,
  /// DNSSEC and DANE
  ///
  /// Method of validating a DNS response against a trusted root server. Mitigates various attacks that could reroute a user to a fake site while showing the real URL for the original site.
  ///
  /// * [Wikipedia - DNSSEC](https://en.wikipedia.org/wiki/Domain_Name_System_Security_Extensions)
  /// * [Chrome implementation bug](https://bugs.chromium.org/p/chromium/issues/detail?id=50874)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=672600)
  Dnssec,
  /// Do Not Track API
  ///
  /// API to allow the browser's Do Not Track setting to be queried via `navigator.doNotTrack`. Due to lack of adoption the Do Not Track specification was deprecated in 2018.
  ///
  /// * [MDN Web Docs - doNotTrack](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/doNotTrack)
  DoNotTrack,
  /// document.currentScript
  ///
  /// `document.currentScript` returns the `<script>` element whose script is currently being processed.
  ///
  /// * [Polyfill (IE 6-10 only)](https://github.com/JamesMGreene/document.currentScript)
  DocumentCurrentscript,
  /// document.evaluate & XPath
  ///
  /// Allow nodes in an XML/HTML document to be traversed using XPath expressions.
  ///
  /// * [XPath in Javascript: Introduction](https://timkadlec.com/2008/02/xpath-in-javascript-introduction/)
  /// * [MDN Web Docs - XPath introduction](https://developer.mozilla.org/en-US/docs/Introduction_to_using_XPath_in_JavaScript)
  /// * [Edge team article on implementation](https://blogs.windows.com/msedgedev/2015/03/19/improving-interoperability-with-dom-l3-xpath/)
  /// * [DOM XPath - WHATWG Wiki](https://wiki.whatwg.org/wiki/DOM_XPath)
  DocumentEvaluateXpath,
  /// Document.execCommand()
  ///
  /// Allows running commands to manipulate the contents of an editable region in a document switched to designMode
  ///
  /// * [MDN Web Docs - execCommand](https://developer.mozilla.org/en-US/docs/Web/API/Document/execCommand)
  /// * [execCommand and queryCommandSupported demo](https://codepen.io/netsi1964/pen/QbLLGW)
  DocumentExeccommand,
  /// Document Policy
  ///
  /// A mechanism that allows developers to set certain rules and policies for a given site. The rules can change default browser behaviour, block certain features or set limits on resource usage. Document Policy is useful both for security and performance, and is similar to [Permissions Policy](/permissions-policy).
  ///
  /// * [Firefox position: non-harmful](https://mozilla.github.io/standards-positions/#document-policy)
  /// * [Chromium tracking bug for new policies](https://bugs.chromium.org/p/chromium/issues/detail?id=993790)
  /// * [WICG - Document Policy Explainer](https://github.com/WICG/document-policy/blob/main/document-policy-explainer.md)
  DocumentPolicy,
  /// document.scrollingElement
  ///
  /// `document.scrollingElement` refers to the element that scrolls the document.
  ///
  /// * [Polyfill](https://github.com/mathiasbynens/document.scrollingElement)
  /// * [MDN on scrollingElement](https://developer.mozilla.org/en-US/docs/Web/API/Document/scrollingElement)
  DocumentScrollingelement,
  /// document.head
  ///
  /// Convenience property for accessing the `<head>` element
  ///
  /// * [MDN Web Docs - head](https://developer.mozilla.org/en-US/docs/Web/API/Document/head)
  Documenthead,
  /// DOM manipulation convenience methods
  ///
  /// jQuery-like methods on DOM nodes to insert nodes around or within a node, or to replace one node with another. These methods accept any number of DOM nodes or HTML strings as arguments. Includes: `ChildNode.before`, `ChildNode.after`, `ChildNode.replaceWith`, `ParentNode.prepend`, and `ParentNode.append`.
  ///
  /// * [WHATWG DOM Specification for ChildNode](https://dom.spec.whatwg.org/#interface-childnode)
  /// * [WHATWG DOM Specification for ParentNode](https://dom.spec.whatwg.org/#interface-parentnode)
  /// * [JS Bin testcase](https://jsbin.com/fiqacod/edit?html,js,output)
  /// * [DOM4 polyfill](https://github.com/WebReflection/dom4)
  DomManipConvenience,
  /// Document Object Model Range
  ///
  /// A contiguous range of content in a Document, DocumentFragment or Attr
  ///
  /// * [MDN Web Docs - Range](https://developer.mozilla.org/en-US/docs/Web/API/Range)
  /// * [QuirksMode](https://www.quirksmode.org/dom/range_intro.html)
  /// * ["Rangy" Range library with old IE support](https://github.com/timdown/rangy)
  DomRange,
  /// DOMContentLoaded
  ///
  /// JavaScript event that fires when the DOM is loaded, but before all page assets are loaded (CSS, images, etc.).
  ///
  /// * [MDN Web Docs - DOMContentLoaded](https://developer.mozilla.org/en-US/docs/Web/Reference/Events/DOMContentLoaded)
  Domcontentloaded,
  /// DOMMatrix
  ///
  /// The `DOMMatrix` interface represents 4x4 matrices, suitable for 2D and 3D operations. Supersedes the `WebKitCSSMatrix` and `SVGMatrix` interfaces.
  ///
  /// * [WebKitCSSMatrix API Reference](https://developer.apple.com/reference/webkitjs/webkitcssmatrix)
  /// * [WebKitCSSMatrix in Compatibility Standard](https://compat.spec.whatwg.org/#webkitcssmatrix-interface)
  /// * [MDN Web Docs - DOMMatrix](https://developer.mozilla.org/en-US/docs/Web/API/DOMMatrix)
  /// * [Chrome implementation bug](https://bugs.chromium.org/p/chromium/issues/detail?id=581955)
  Dommatrix,
  /// Download attribute
  ///
  /// When used on an anchor, this attribute signifies that the browser should download the resource the anchor points to rather than navigate to it.
  ///
  /// * [HTML5Rocks post](https://updates.html5rocks.com/2011/08/Downloading-resources-in-HTML5-a-download)
  /// * [IE11 polyfill](https://github.com/jelmerdemaat/dwnld-attr-polyfill)
  /// * [Download attribute on MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a#attr-download)
  Download,
  /// Drag and Drop
  ///
  /// Method of easily dragging and dropping elements on a page, requiring minimal JavaScript.
  ///
  /// * [HTML5 Doctor article](https://html5doctor.com/native-drag-and-drop/)
  /// * [Shopping cart demo](https://nettutsplus.s3.amazonaws.com/64_html5dragdrop/demo/index.html)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/DragEvent)
  /// * [Polyfill for setDragImage in IE](https://github.com/MihaiValentin/setDragImage-IE)
  /// * [iOS/Android shim for HTML 5 drag'n'drop](https://github.com/timruffles/ios-html5-drag-drop-shim)
  Dragndrop,
  /// Element.closest()
  ///
  /// DOM method that returns the current element if it matches the given selector, or else the closest ancestor element that matches the given selector, or else null.
  ///
  /// * [MDN Web Docs - closest](https://developer.mozilla.org/en-US/docs/Web/API/Element/closest)
  /// * [Polyfill](https://github.com/jonathantneal/closest)
  ElementClosest,
  /// document.elementFromPoint()
  ///
  /// Given coordinates for a point relative to the viewport, returns the element that a click event would be dispatched at if the user were to click the point (in other words, the element that hit-testing would find).
  ///
  /// * [MDN Web Docs - elementFromPoint](https://developer.mozilla.org/en-US/docs/Web/API/Document/elementFromPoint)
  ElementFromPoint,
  /// Scroll methods on elements (scroll, scrollTo, scrollBy)
  ///
  /// Methods to change the scroll position of an element. Similar to setting `scrollTop` & `scrollLeft` properties, but also allows options to be passed to define the scroll behavior.
  ///
  /// * [MDN article on scrollTo](https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollTo)
  /// * [MDN article on scrollBy](https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollBy)
  ElementScrollMethods,
  /// Encrypted Media Extensions
  ///
  /// The EncryptedMediaExtenstions API provides interfaces for controlling the playback of content which is subject to a DRM scheme.
  ///
  /// * [HTML5rocks article](https://www.html5rocks.com/en/tutorials/eme/basics/)
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Encrypted_Media_Extensions)
  /// * [Encrypted Media Extensions API on MDN](https://developer.mozilla.org/en-US/docs/Web/API/Encrypted_Media_Extensions_API)
  Eme,
  /// EOT - Embedded OpenType fonts
  ///
  /// Type of font that can be derived from a regular font, allowing small files and legal use of high-quality fonts. Usage is restricted by the file being tied to the website
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/Embedded_OpenType)
  /// * [Example pages](https://www.microsoft.com/typography/web/embedding/default.aspx)
  Eot,
  /// ECMAScript 5
  ///
  /// Full support for the ECMAScript 5 specification. Features include `Function.prototype.bind`, Array methods like `indexOf`, `forEach`, `map` & `filter`, Object methods like `defineProperty`, `create` & `keys`, the `trim` method on Strings and many more.
  ///
  /// * [Detailed compatibility tables & tests](https://compat-table.github.io/compat-table/es5/)
  /// * [Overview of objects & properties](https://johnresig.com/blog/ecmascript-5-objects-and-properties/)
  /// * [ES5 polyfill](https://github.com/es-shims/es5-shim)
  /// * [Polyfill for all possible ES5 features is available in the core-js library](https://github.com/zloirock/core-js#ecmascript)
  Es5,
  /// ECMAScript 2015 (ES6)
  ///
  /// Support for the ECMAScript 2015 specification. Features include Promises, Modules, Classes, Template Literals, Arrow Functions, Let and Const, Default Parameters, Generators, Destructuring Assignment, Rest & Spread, Map/Set & WeakMap/WeakSet and many more.
  ///
  /// * [ES6 New features: overview and comparisons](http://es6-features.org)
  /// * [Exploring ES6 (book)](https://exploringjs.com/es6/)
  /// * [Polyfill for all possible ES2015 features is available in the core-js library](https://github.com/zloirock/core-js#ecmascript)
  Es6,
  /// ES6 classes
  ///
  /// ES6 classes are syntactical sugar to provide a much simpler and clearer syntax to create objects and deal with inheritance.
  ///
  /// * [MDN Web Docs - ES6 classes](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Classes)
  /// * [Sitepoint deep dive on ES6 classes](https://www.sitepoint.com/object-oriented-javascript-deep-dive-es6-classes/)
  /// * [List of resources critical of ES6 classes](https://github.com/joshburgess/not-awesome-es6-classes)
  Es6Class,
  /// ES6 Generators
  ///
  /// ES6 Generators are special functions that can be used to control the iteration behavior of a loop. Generators are defined using a `function*` declaration.
  ///
  /// * [Exploring JS chapter on generators](https://exploringjs.com/es6/ch_generators.html)
  /// * [MDN article on the `function*` declaration](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/function*)
  Es6Generators,
  /// JavaScript modules via script tag
  ///
  /// Loading JavaScript module scripts (aka ES6 modules) using `<script type="module">` Includes support for the `nomodule` attribute.
  ///
  /// * [Intro to ES6 modules](https://strongloop.com/strongblog/an-introduction-to-javascript-es6-modules/)
  /// * [MS Edge blog post](https://blogs.windows.com/msedgedev/2016/05/17/es6-modules-and-beyond/)
  /// * [Mozilla hacks article](https://hacks.mozilla.org/2015/08/es6-in-depth-modules/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=568953)
  /// * [Blog post: Native ECMAScript modules - the first overview](https://blog.hospodarets.com/native-ecmascript-modules-the-first-overview)
  /// * [Counterpart ECMAScript specification for import/export syntax](https://tc39.es/ecma262/#sec-modules)
  /// * [Specification for nomodule attribute](https://html.spec.whatwg.org/multipage/scripting.html#attr-script-nomodule)
  /// * [Blog post on using nomodule](https://hospodarets.com/native-ecmascript-modules-nomodule)
  /// * [Will it double-fetch? Browser behavior with `module` / `nomodule` scripts](https://gist.github.com/jakub-g/5fc11af85a061ca29cc84892f1059fec)
  Es6Module,
  /// JavaScript modules: dynamic import()
  ///
  /// Loading JavaScript modules dynamically using the import() syntax
  ///
  /// * [Counterpart ECMAScript specification for import() syntax](https://tc39.es/ecma262/#sec-import-calls)
  /// * [Blog post: Native ECMAScript modules - dynamic import()](https://hospodarets.com/native-ecmascript-modules-dynamic-import)
  /// * [Dynamic import()](https://developers.google.com/web/updates/2017/11/dynamic-import)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1342012)
  /// * [Integration with the HTML specification](https://html.spec.whatwg.org/multipage/webappapis.html#integration-with-the-javascript-module-system)
  Es6ModuleDynamicImport,
  /// ES6 Number
  ///
  /// Extensions to the `Number` built-in object in ES6, including constant properties `EPSILON`, `MIN_SAFE_INTEGER`, and `MAX_SAFE_INTEGER`, and methods ` isFinite`, `isInteger`, `isSafeInteger`, and `isNaN`.
  ///
  /// * [New number and Math features in ES6](https://2ality.com/2015/04/numbers-math-es6.html)
  /// * [Polyfill for those features is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-number)
  Es6Number,
  /// String.prototype.includes
  ///
  /// The includes() method determines whether one string may be found within another string, returning true or false as appropriate.
  ///
  /// * [MDN: String.prototype.includes()](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/includes)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-string-and-regexp)
  Es6StringIncludes,
  /// Server-sent events
  ///
  /// Method of continuously sending data from a server to the browser, rather than repeatedly requesting it (EventSource interface, used to fall under HTML5)
  ///
  /// * [HTML5 Rocks tutorial](https://www.html5rocks.com/tutorials/eventsource/basics/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-eventsource)
  /// * [Polyfill](https://github.com/Yaffle/EventSource)
  Eventsource,
  /// ui-serif, ui-sans-serif, ui-monospace and ui-rounded values for font-family
  ///
  /// Allows more control when choosing system interface fonts
  ///
  /// * [WebKit Safari 13.1 announcement](https://webkit.org/blog/10247/new-webkit-features-in-safari-13-1/)
  /// * [ui-serif Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1598879)
  /// * [ui-sans-serif Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1598880)
  /// * [ui-monospace Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1598881)
  /// * [ui-rounded Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1598883)
  /// * [Chromium support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1029069)
  ExtendedSystemFonts,
  /// Feature Policy
  ///
  /// This specification defines a mechanism that allows developers to selectively enable and disable use of various browser features and APIs. Feature Policy is deprecated and has been replaced with [Permissions Policy](/permissions-policy) and [Document Policy](/document-policy).
  ///
  /// * [Feature Policy Kitchen Sink Demos](https://feature-policy-demos.appspot.com/)
  /// * [Introduction to Feature Policy](https://developers.google.com/web/updates/2018/06/feature-policy)
  /// * [Firefox implementation ticket](https://bugzilla.mozilla.org/show_bug.cgi?id=1390801)
  /// * [Feature Policy Tester (Chrome DevTools Extension)](https://chrome.google.com/webstore/detail/feature-policy-tester-dev/pchamnkhkeokbpahnocjaeednpbpacop)
  /// * [featurepolicy.info (Feature-Policy Playground)](https://featurepolicy.info/)
  /// * [List of known features](https://github.com/w3c/webappsec-permissions-policy/blob/main/features.md)
  FeaturePolicy,
  /// Fetch
  ///
  /// A modern replacement for XMLHttpRequest.
  ///
  /// * [Polyfill](https://github.com/github/fetch)
  /// * [Demo](https://addyosmani.com/demos/fetch-api/)
  /// * [Polyfill (minimal, 500 bytes)](https://github.com/developit/unfetch)
  Fetch,
  /// disabled attribute of the fieldset element
  ///
  /// Allows disabling all of the form control descendants of a fieldset via a `disabled` attribute on the fieldset element itself.
  ///
  /// * [JS Bin Testcase/Demo](https://jsbin.com/bibiqi/1/edit?html,output)
  FieldsetDisabled,
  /// File API
  ///
  /// Method of manipulating file objects in web applications client-side, as well as programmatically selecting them and accessing their data.
  ///
  /// * [MDN Web Docs - Using Files](https://developer.mozilla.org/en/Using_files_from_web_applications)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/file)
  /// * [Polyfill](https://github.com/moxiecode/moxie)
  Fileapi,
  /// FileReader API
  ///
  /// Method of reading the contents of a File or Blob object into memory
  ///
  /// * [MDN Web Docs - FileReader](https://developer.mozilla.org/en/DOM/FileReader)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/file/FileReader)
  Filereader,
  /// FileReaderSync
  ///
  /// Allows files to be read synchronously in Web Workers
  ///
  /// * [MDN Web Docs - FileReaderSync](https://developer.mozilla.org/en-US/docs/Web/API/FileReaderSync)
  Filereadersync,
  /// Filesystem & FileWriter API
  ///
  /// Method of reading and writing files to a sandboxed file system.
  ///
  /// * [HTML5 Rocks tutorial](https://www.html5rocks.com/en/tutorials/file/filesystem/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/filesystem)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=997471)
  Filesystem,
  /// FLAC audio format
  ///
  /// Popular lossless audio compression format
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/FLAC)
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=93887)
  Flac,
  /// CSS Flexible Box Layout Module
  ///
  /// Method of positioning elements in horizontal or vertical stacks. Support includes all properties prefixed with `flex`, as well as `display: flex`, `display: inline-flex`, `align-content`, `align-items`, `align-self`, `justify-content` and `order`.
  ///
  /// * [Flexbox CSS generator](https://bennettfeely.com/flexplorer/)
  /// * [Article on using the latest spec](https://www.adobe.com/devnet/html5/articles/working-with-flexbox-the-new-spec.html)
  /// * [Tutorial on cross-browser support](https://dev.opera.com/articles/view/advanced-cross-browser-flexbox/)
  /// * [Examples on how to solve common layout problems with flexbox](https://philipwalton.github.io/solved-by-flexbox/)
  /// * [A Complete Guide to Flexbox](https://css-tricks.com/snippets/css/a-guide-to-flexbox/)
  /// * [Flexbox playground and code generator](https://the-echoplex.net/flexyboxes/)
  /// * [Flexbugs: Repo for flexbox bugs](https://github.com/philipwalton/flexbugs)
  /// * [10up Open Sources IE 8 and 9 Support for Flexbox](https://github.com/10up/flexibility/)
  /// * [Ecligrid - Mobile first flexbox grid system](https://github.com/vadimyer/ecligrid)
  /// * [The Difference Between Width and Flex-Basis](https://mastery.games/post/the-difference-between-width-and-flex-basis/)
  Flexbox,
  /// gap property for Flexbox
  ///
  /// `gap` for flexbox containers to create gaps/gutters between flex items
  ///
  /// * [Spec discussion](https://github.com/w3c/csswg-drafts/issues/592)
  /// * [Chrome bug to track implementation](https://bugs.chromium.org/p/chromium/issues/detail?id=762679)
  /// * [MDN browser compatibility](https://developer.mozilla.org/en-US/docs/Web/CSS/gap#Browser_compatibility)
  /// * [Workaround using negative margins](https://gist.github.com/OliverJAsh/7f29d0fa1d35216ec681d2949c3fe8b7)
  /// * [Webkit support bug](https://bugs.webkit.org/show_bug.cgi?id=206767)
  FlexboxGap,
  /// display: flow-root
  ///
  /// The element generates a block container box, and lays out its contents using flow layout. It always establishes a new block formatting context for its contents. It provides a better solution to the most use cases of the "clearfix" hack.
  ///
  /// * [Mozilla bug report](https://bugzilla.mozilla.org/show_bug.cgi?id=1322191)
  /// * [Chromium bug report](https://bugs.chromium.org/p/chromium/issues/detail?id=672508)
  /// * [WebKit bug report](https://bugs.webkit.org/show_bug.cgi?id=165603)
  /// * [Blog post: "The end of the clearfix hack?"](https://rachelandrew.co.uk/archives/2017/01/24/the-end-of-the-clearfix-hack/)
  FlowRoot,
  /// focusin & focusout events
  ///
  /// The `focusin` and `focusout` events fire just before the element gains or loses focus, and they bubble. By contrast, the `focus` and `blur` events fire after the focus has shifted, and don't bubble.
  ///
  /// * [MDN Web Docs - focusin](https://developer.mozilla.org/en-US/docs/Web/Events/focusin)
  /// * [MDN Web Docs - focusout](https://developer.mozilla.org/en-US/docs/Web/Events/focusout)
  /// * [Mozilla Bug 687787 - Add support for DOM3 focusin/focusout](https://bugzilla.mozilla.org/show_bug.cgi?id=687787)
  FocusinFocusoutEvents,
  /// system-ui value for font-family
  ///
  /// Value for `font-family` that represents the default user interface font.
  ///
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1226042)
  /// * [MDN on the font-family property](https://developer.mozilla.org/en-US/docs/Web/CSS/font-family)
  FontFamilySystemUi,
  /// CSS font-feature-settings
  ///
  /// Method of applying advanced typographic and language-specific font features to supported OpenType fonts.
  ///
  /// * [Demo pages (IE/Firefox only)](https://testdrive-archive.azurewebsites.net/Graphics/opentype/)
  /// * [Mozilla hacks article](https://hacks.mozilla.org/2010/11/firefox-4-font-feature-support/)
  /// * [Detailed tables on accessibility support](https://html5accessibility.com/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/font-feature-settings)
  /// * [MDN Web Docs - font-feature-settings](https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings)
  /// * [OpenType layout feature tag registry](https://www.microsoft.com/typography/otspec/featuretags.htm)
  /// * [Syntax for OpenType features in CSS (Adobe Typekit Help)](https://helpx.adobe.com/fonts/using/open-type-syntax.html#salt)
  FontFeature,
  /// CSS3 font-kerning
  ///
  /// Controls the usage of the kerning information (spacing between letters) stored in the font. Note that this only affects OpenType fonts with kerning information, it has no effect on other fonts.
  ///
  /// * [MDN Web Docs - CSS font-kerning](https://developer.mozilla.org/en-US/docs/Web/CSS/font-kerning)
  FontKerning,
  /// CSS Font Loading
  ///
  /// This CSS module defines a scripting interface to font faces in CSS, allowing font faces to be easily created and loaded from script. It also provides methods to track the loading status of an individual font, or of all the fonts on an entire page.
  ///
  /// * [Optimizing with font load events](https://www.igvita.com/2014/01/31/optimizing-web-font-rendering-performance/#font-load-events)
  FontLoading,
  /// CSS font-size-adjust
  ///
  /// Method of adjusting the font size in a matter that relates to the height of lowercase vs. uppercase letters. This makes it easier to set the size of fallback fonts.
  ///
  /// * [Article on font-size-adjust](https://webdesignernotebook.com/css/the-little-known-font-size-adjust-css3-property/)
  /// * [MDN Web Docs - CSS font-size-adjust](https://developer.mozilla.org/en-US/docs/Web/CSS/font-size-adjust)
  /// * [WebKit support bug #15257](https://bugs.webkit.org/show_bug.cgi?id=15257)
  FontSizeAdjust,
  /// CSS font-smooth
  ///
  /// Controls the application of anti-aliasing when fonts are rendered.
  ///
  /// * [MDN Web Docs - font-smooth](https://developer.mozilla.org/en-US/docs/Web/CSS/font-smooth)
  /// * [Old version of W3C recommendation containing font-smooth](https://www.w3.org/TR/WD-font/#font-smooth)
  /// * [WHATWG compat issue to spec `-webkit-font-smoothing: antialiased`](https://github.com/whatwg/compat/issues/115)
  FontSmooth,
  /// Font unicode-range subsetting
  ///
  /// This @font-face descriptor defines the set of Unicode codepoints that may be supported by the font face for which it is declared. The descriptor value is a comma-delimited list of Unicode range (<urange>) values. The union of these ranges defines the set of codepoints that serves as a hint for user agents when deciding whether or not to download a font resource for a given text run.
  ///
  /// * [MDN Web Docs - CSS unicode-range](https://developer.mozilla.org/en-US/docs/Web/CSS/unicode-range)
  /// * [Safari CSS Reference: unicode-range](https://developer.apple.com/library/safari/documentation/AppleApplications/Reference/SafariCSSRef/Articles/StandardCSSProperties.html#//apple_ref/css/property/unicode-range)
  /// * [Web Platform Docs: unicode-range](https://webplatform.github.io/docs/css/properties/unicode-range)
  /// * [Demo](https://jsbin.com/jeqoguzeye/1/edit?html,output)
  FontUnicodeRange,
  /// CSS font-variant-alternates
  ///
  /// Controls the usage of alternate glyphs associated to alternative names defined in @font-feature-values for certain types of OpenType fonts.
  ///
  /// * [MDN Web Docs - font-variant-alternates](https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-alternates)
  /// * [Chromium support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=716567)
  FontVariantAlternates,
  /// CSS font-variant-numeric
  ///
  /// CSS property that provides different ways of displaying numbers, fractions, and ordinal markers.
  ///
  /// * [MDN Web Docs article](https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-numeric)
  FontVariantNumeric,
  /// @font-face Web fonts
  ///
  /// Method of displaying fonts downloaded from websites
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/Web_typography)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/atrules/font-face)
  Fontface,
  /// Form attribute
  ///
  /// Attribute for associating input and submit buttons with a form.
  ///
  /// * [Input attribute specification](https://www.w3.org/TR/html5/forms.html#attr-fae-form)
  /// * [Article on usage](https://www.impressivewebs.com/html5-form-attribute/)
  FormAttribute,
  /// Attributes for form submission
  ///
  /// Attributes for form submission that may be specified on submit buttons. The attributes are: `formaction`, `formenctype`, `formmethod`, `formnovalidate`, and `formtarget`
  ///
  /// * [Article describing each attribute](https://html5doctor.com/html5-forms-introduction-and-new-attributes/#formaction)
  FormSubmitAttributes,
  /// Form validation
  ///
  /// Method of setting required fields and field types without requiring JavaScript. This includes preventing forms from being submitted when appropriate, the `checkValidity()` method as well as support for the `:invalid`, `:valid`, and `:required` CSS pseudo-classes.
  ///
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/attributes/required)
  /// * [WebKit Blog: HTML Interactive Form Validation](https://webkit.org/blog/7099/html-interactive-form-validation/)
  FormValidation,
  /// Fullscreen API
  ///
  /// API for allowing content (like a video or canvas element) to take up the entire screen.
  ///
  /// * [MDN Web Docs - Using Full Screen](https://developer.mozilla.org/en/DOM/Using_full-screen_mode)
  /// * [Mozilla hacks article](https://hacks.mozilla.org/2012/01/using-the-fullscreen-api-in-web-browsers/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/Element/requestFullscreen)
  Fullscreen,
  /// Gamepad API
  ///
  /// API to support input from USB gamepad controllers through JavaScript.
  ///
  /// * [Controller demo](https://luser.github.io/gamepadtest/)
  /// * [MDN Web Docs - Gamepad](https://developer.mozilla.org/en-US/docs/Web/API/Gamepad_API)
  /// * [HTML5Rocks article](https://www.html5rocks.com/en/tutorials/doodles/gamepad/)
  /// * [Detailed tutorial](https://gamedevelopment.tutsplus.com/tutorials/using-the-html5-gamepad-api-to-add-controller-support-to-browser-games--cms-21345)
  Gamepad,
  /// Geolocation
  ///
  /// Method of informing a website of the user's geographical location
  ///
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-geolocation)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/geolocation)
  /// * [Geolocation API on MDN](https://developer.mozilla.org/en-US/docs/Web/API/Geolocation_API)
  Geolocation,
  /// Element.getBoundingClientRect()
  ///
  /// Method to get the size and position of an element's bounding box, relative to the viewport.
  ///
  /// * [MDN Web Docs - getBoundingClientRect](https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect)
  /// * [Microsoft Developer Network](https://msdn.microsoft.com/en-us/library/ms536433(VS.85).aspx)
  Getboundingclientrect,
  /// getComputedStyle
  ///
  /// API to get the current computed CSS styles applied to an element. This may be the current value applied by an animation or as set by a stylesheet.
  ///
  /// * [MDN Web Docs - getComputedStyle](https://developer.mozilla.org/en/DOM/window.getComputedStyle)
  /// * [Demo](https://testdrive-archive.azurewebsites.net/HTML5/getComputedStyle/)
  /// * [Polyfill for IE](https://snipplr.com/view/13523)
  Getcomputedstyle,
  /// getElementsByClassName
  ///
  /// Method of accessing DOM elements by class name
  ///
  /// * [Test page](https://www.quirksmode.org/dom/tests/basics.html#getElementsByClassName)
  /// * [getElementsByClassName on MDN](https://developer.mozilla.org/en-US/docs/Web/API/Element/getElementsByClassName)
  Getelementsbyclassname,
  /// crypto.getRandomValues()
  ///
  /// Method of generating cryptographically random values.
  ///
  /// * [MDN Web Docs - crypto.getRandomValues](https://developer.mozilla.org/en-US/docs/Web/API/window.crypto.getRandomValues)
  Getrandomvalues,
  /// Gyroscope
  ///
  /// Defines a concrete sensor interface to monitor the rate of rotation around the device’s local three primary axes.
  ///
  /// * [Demo](https://intel.github.io/generic-sensor-demos/)
  /// * [Article](https://developers.google.com/web/updates/2017/09/sensors-for-the-web#gyroscope-sensor)
  Gyroscope,
  /// navigator.hardwareConcurrency
  ///
  /// Returns the number of logical cores of the user's CPU. The value may be reduced to prevent device fingerprinting or because it exceeds the allowed number of simultaneous web workers.
  ///
  /// * [MDN Web Docs - navigator.hardwareConcurrency](https://developer.mozilla.org/en-US/docs/Web/API/NavigatorConcurrentHardware/hardwareConcurrency)
  /// * [Original Proposal](https://wiki.whatwg.org/wiki/Navigator_HW_Concurrency)
  /// * [WebKit implementation bug](https://bugs.webkit.org/show_bug.cgi?id=132588)
  Hardwareconcurrency,
  /// Hashchange event
  ///
  /// Event triggered in JavaScript when the URL's hash has changed (for example: page.html#foo to page.html#bar)
  ///
  /// * [MDN Web Docs - onhashchange](https://developer.mozilla.org/en-US/docs/Web/API/Window/hashchange_event)
  /// * [Simple demo](https://www.quirksmode.org/dom/events/tests/hashchange.html)
  /// * [Polyfill](https://github.com/3nr1c/jUri.js)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/Element/hashchange)
  Hashchange,
  /// HEIF/HEIC image format
  ///
  /// A modern image format based on the [HEVC video format](/hevc). HEIC generally has better compression than [WebP](/webp), JPEG, PNG and GIF. It is hard for browsers to support HEIC because it is [complex and expensive to license](https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding#Patent_licensing). [AVIF](/avif) and [JPEG XL](/jpegxl) provide free licenses and are designed to supersede HEIC.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/High_Efficiency_Image_File_Format)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=HEIF)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=230035)
  Heif,
  /// HEVC/H.265 video format
  ///
  /// The High Efficiency Video Coding (HEVC) compression standard is a video compression format intended to succeed H.264. It is hard for browsers to universally support HEVC because it is [complex and expensive to license](https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding#Patent_licensing). HEVC competes with [AV1](/av1) which has similar compression quality and provides a free license.
  ///
  /// * [Firefox support bug (WONTFIX)](https://bugzilla.mozilla.org/show_bug.cgi?format=default&id=1332136)
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/High_Efficiency_Video_Coding)
  /// * [Chrome support bug (WontFix)](https://bugs.chromium.org/p/chromium/issues/detail?id=684382)
  /// * [Firefox support via OS API bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1842838)
  Hevc,
  /// hidden attribute
  ///
  /// The `hidden` attribute may be applied to any element, and effectively hides elements similar to `display: none` in CSS.
  ///
  /// * [Article on hidden attribute](https://davidwalsh.name/html5-hidden)
  Hidden,
  /// High Resolution Time API
  ///
  /// Method to provide the current time in sub-millisecond resolution and such that it is not subject to system clock skew or adjustments. Called using `performance.now()`
  ///
  /// * [MDN Web Docs - Performance.now](https://developer.mozilla.org/en-US/docs/Web/API/Performance.now())
  /// * [HTML5Rocks article](https://developer.chrome.com/blog/when-milliseconds-are-not-enough-performance-now/)
  /// * [SitePoint article](https://www.sitepoint.com/discovering-the-high-resolution-time-api/)
  /// * [Demo](https://audero.it/demo/high-resolution-time-api-demo.html)
  HighResolutionTime,
  /// Session history management
  ///
  /// Method of manipulating the user's browser's session history in JavaScript using `history.pushState`, `history.replaceState` and the `popstate` event.
  ///
  /// * [Introduction to history management](https://www.adequatelygood.com/Saner-HTML5-History-Management.html)
  /// * [MDN Web Docs - Manipulating the browser history](https://developer.mozilla.org/en/DOM/Manipulating_the_browser_history)
  /// * [Demo page](https://html5demos.com/history)
  /// * [History.js polyfill](https://github.com/browserstate/history.js)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-history-state)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/History)
  History,
  /// HTML Media Capture
  ///
  /// Facilitates user access to a device's media capture mechanism, such as a camera, or microphone, from within a file upload control.
  ///
  /// * [Correct Syntax for HTML Media Capture](https://addpipe.com/blog/correct-syntax-html-media-capture/)
  /// * [Programming the Mobile Web: File upload compatibility table](https://books.google.com.au/books?id=gswdarRZVUoC&pg=PA263&dq=%22file+upload+compatibility+table%22)
  /// * [HTML Media Capture Test Bench](https://addpipe.com/html-media-capture-demo/)
  HtmlMediaCapture,
  /// HTML5 semantic elements
  ///
  /// HTML5 offers some new elements, primarily for semantic purposes. The elements include: `section`, `article`, `aside`, `header`, `footer`, `nav`, `figure`, `figcaption`, `time`, `mark` & `main`.
  ///
  /// * [Workaround for IE](https://blog.whatwg.org/supporting-new-elements-in-ie)
  /// * [Alternate workaround](https://blog.whatwg.org/styling-ie-noscript)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/dom.js#dom-html5-elements)
  /// * [Chrome Platform Status: `<time>` element](https://www.chromestatus.com/feature/5633937149788160)
  Html5semantic,
  /// HTTP Live Streaming (HLS)
  ///
  /// HTTP-based media streaming communications protocol
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/HTTP_Live_Streaming)
  /// * [Apple developer article](https://developer.apple.com/streaming/)
  HttpLiveStreaming,
  /// HTTP/2 protocol
  ///
  /// Networking protocol for low-latency transport of content over the web. Originally started out from the SPDY protocol, now standardized as HTTP version 2.
  ///
  /// * [Wikipedia article about HTTP/2](https://en.wikipedia.org/wiki/HTTP/2)
  /// * [Browser support test](https://http2.akamai.com/demo)
  Http2,
  /// HTTP/3 protocol
  ///
  /// Third version of the HTTP networking protocol which uses QUIC as transport protocol. Previously known as HTTP-over-QUIC, now standardized as HTTP/3.
  ///
  /// * [Wikipedia article about HTTP/3](https://en.wikipedia.org/wiki/HTTP/3)
  Http3,
  /// sandbox attribute for iframes
  ///
  /// Method of running external site pages with reduced privileges (e.g. no JavaScript) in iframes.
  ///
  /// * [Chromium blog article](https://blog.chromium.org/2010/05/security-in-depth-html5s-sandbox.html)
  /// * [MSDN article](https://msdn.microsoft.com/en-us/hh563496)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/attributes/sandbox)
  IframeSandbox,
  /// seamless attribute for iframes
  ///
  /// The seamless attribute makes an iframe's contents actually part of a page, and adopts the styles from its hosting page. The attribute has been removed from both [the WHATWG](https://github.com/whatwg/html/issues/331) and [the W3C](https://github.com/w3c/html/pull/325) HTML5 specifications.
  ///
  /// * [Experimental polyfill](https://github.com/ornj/seamless-polyfill)
  /// * [Article](https://labs.ft.com/2013/01/seamless-iframes-not-quite-seamless/)
  /// * [Bug on Firefox support: wontfix](https://bugzilla.mozilla.org/show_bug.cgi?id=631218)
  IframeSeamless,
  /// srcdoc attribute for iframes
  ///
  /// Override the content specified in the `src` attribute (if present) with HTML content within the attribute.
  ///
  /// * [MDN Web Docs - iframe](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
  /// * [Srcdoc Polyfill](https://github.com/jugglinmike/srcdoc-polyfill)
  /// * [Article](https://bocoup.com/weblog/third-party-javascript-development-future/)
  IframeSrcdoc,
  /// ImageCapture API
  ///
  /// The Image Capture API provides access to the Video Camera for taking photos while configuring picture-specific settings such as e.g. zoom or auto focus metering area.
  ///
  /// * [Minimal code pen](https://codepen.io/miguelao/pen/ZOkOQw)
  /// * [Extended demo](https://rawgit.com/Miguelao/demos/master/imagecapture.html)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=888177)
  Imagecapture,
  /// Input Method Editor API
  ///
  /// Provides scripted access to the Input Method Editor (IME). An IME is often used to input characters from East Asian languages by typing roman characters and selecting from the resulting suggestions.
  ///
  /// * [Building Better Input Experience for East Asian Users with the IME API in IE11](https://web.archive.org/web/20140403042251/http://blogs.msdn.com/b/ie/archive/2014/03/31/building-better-input-experience-for-east-asian-users-with-the-ime-api-in-ie11.aspx)
  Ime,
  /// naturalWidth & naturalHeight image properties
  ///
  /// Properties defining the intrinsic width and height of the image, rather than the displayed width & height.
  ///
  /// * [Blog post on support in IE](https://www.jacklmoore.com/notes/naturalwidth-and-naturalheight-in-ie/)
  /// * [gist on getting natural width & height in older IE](https://gist.github.com/jalbertbowden/5273983)
  ImgNaturalwidthNaturalheight,
  /// Import maps
  ///
  /// Import maps allow control over what URLs get fetched by JavaScript `import` statements and `import()` expressions.
  ///
  /// * [Proposal information](https://github.com/WICG/import-maps#readme)
  /// * [Using ES modules in browsers with import-maps](https://blog.logrocket.com/es-modules-in-browsers-with-import-maps/)
  /// * [Firefox feature request bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1688879)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=220823)
  ImportMaps,
  /// HTML Imports
  ///
  /// Deprecated method of including and reusing HTML documents in other HTML documents. Superseded by ES modules.
  ///
  /// * [HTML5Rocks - HTML Imports: #include for the web](https://www.html5rocks.com/tutorials/webcomponents/imports/)
  /// * [Chromium tracking bug: Implement HTML Imports](https://code.google.com/p/chromium/issues/detail?id=240592)
  /// * [Firefox tracking bug: Implement HTML Imports](https://bugzilla.mozilla.org/show_bug.cgi?id=877072)
  /// * [IE Web Platform Status and Roadmap: HTML Imports](https://developer.microsoft.com/en-us/microsoft-edge/status/htmlimports/)
  Imports,
  /// indeterminate checkbox
  ///
  /// Indeterminate checkboxes are displayed in a state which is distinct both from being checked or being unchecked. They are commonly used in hierarchical checkboxes to indicate that only some of the checkbox's descendants are checked.
  ///
  /// * [CSS-Tricks article](https://css-tricks.com/indeterminate-checkboxes/)
  /// * [iOS versions below 12 don't support indeterminate checkboxes (WebKit Bug 160484)](https://bugs.webkit.org/show_bug.cgi?id=160484)
  IndeterminateCheckbox,
  /// IndexedDB
  ///
  /// Method of storing data client-side, allows indexed database queries.
  ///
  /// * [Mozilla Hacks article](https://hacks.mozilla.org/2010/06/comparing-indexeddb-and-webdatabase/)
  /// * [Polyfill for browsers supporting WebSQL](https://github.com/axemclion/IndexedDBShim)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-indexeddb)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/indexeddb)
  Indexeddb,
  /// IndexedDB 2.0
  ///
  /// Improvements to Indexed DB, including getAll(), renaming stores and indexes, and binary keys.
  ///
  /// * [Mozilla Hacks: What's new in IndexedDB 2.0?](https://hacks.mozilla.org/2016/10/whats-new-in-indexeddb-2-0/)
  /// * [MDN Web Docs - IndexedDB API](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)
  Indexeddb2,
  /// CSS inline-block
  ///
  /// Method of displaying an element as a block while flowing it with text.
  ///
  /// * [Blog post w/info](https://robertnyman.com/2010/02/24/css-display-inline-block-why-it-rocks-and-why-it-sucks/)
  /// * [Info on cross browser support](https://blog.mozilla.org/webdev/2009/02/20/cross-browser-inline-block/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/display)
  InlineBlock,
  /// HTMLElement.innerText
  ///
  /// A property representing the text within a DOM element and its descendants. As a getter, it approximates the text the user would get if they highlighted the contents of the element with the cursor and then copied to the clipboard.
  ///
  /// * [MDN Web Docs - innerText](https://developer.mozilla.org/en-US/docs/Web/API/HTMLElement/innerText)
  /// * [WHATWG Compatibility Standard issue #5: spec innerText](https://github.com/whatwg/compat/issues/5)
  /// * [Rangy, a JS range and selection library which contains an innerText implementation](https://github.com/timdown/rangy)
  /// * [Standardizing innerText – Web Incubator Community Group (WICG) discussion](https://discourse.wicg.io/t/standardizing-innertext/799)
  Innertext,
  /// autocomplete attribute: on & off values
  ///
  /// The `autocomplete` attribute for `input` elements indicates to the browser whether a value should or should not be autofilled when appropriate.
  ///
  /// * [MDN Web Docs - autocomplete attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input#attr-autocomplete)
  InputAutocompleteOnoff,
  /// Color input type
  ///
  /// Form field allowing the user to select a color.
  ///
  /// * [Polyfill](https://github.com/jonstipe/color-polyfill)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/input/type/color)
  /// * [MDN web docs](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/color)
  InputColor,
  /// Date and time input types
  ///
  /// Form field widgets to easily allow users to enter a date, time or both, generally by using a calendar/time input widget. Refers to supporting the following input types: `date`, `time`, `datetime-local`, `month` & `week`.
  ///
  /// * [Datepicker tutorial w/polyfill](https://code.tutsplus.com/tutorials/quick-tip-create-cross-browser-datepickers-in-minutes--net-20236)
  /// * [Polyfill for HTML5 forms](https://github.com/zoltan-dulac/html5Forms.js)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/form.js#input-type-datetime-local)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/input/type/date)
  /// * [Bug on Firefox support](https://bugzilla.mozilla.org/show_bug.cgi?id=888320)
  /// * [Bug for WebKit/Safari](https://bugs.webkit.org/show_bug.cgi?id=119175)
  /// * [Bug for WebKit/Safari](https://bugs.webkit.org/show_bug.cgi?id=214946)
  InputDatetime,
  /// Email, telephone & URL input types
  ///
  /// Text input fields intended for email addresses, telephone numbers or URLs. Particularly useful in combination with [form validation](https://caniuse.com/#feat=form-validation)
  ///
  /// * [Article on usage](https://www.htmlgoodies.com/guides/html5-forms-how-to-use-the-new-email-url-and-telephone-input-types/#fbid=c9PEy7_9RZb)
  InputEmailTelUrl,
  /// input event
  ///
  /// The `input` event is fired when the user changes the value of an `<input>` element, `<select>` element, or `<textarea>` element. By contrast, the "change" event usually only fires after the form control has lost focus.
  ///
  /// * [Specification for `<select>` elements firing the `input` event](https://html.spec.whatwg.org/multipage/forms.html#send-select-update-notifications)
  /// * [MDN Web Docs - input event](https://developer.mozilla.org/en-US/docs/Web/Events/input)
  InputEvent,
  /// accept attribute for file input
  ///
  /// Allows a filter to be defined for what type of files a user may pick with from an `<input type="file">` dialog
  ///
  /// * [Demo & information](https://www.wufoo.com/html5/attributes/07-accept.html)
  InputFileAccept,
  /// Directory selection from file input
  ///
  /// The `webkitdirectory` attribute on the `<input type="file">` element allows entire directory with file contents (and any subdirectories) to be selected.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/API/HTMLInputElement/webkitdirectory)
  InputFileDirectory,
  /// Multiple file selection
  ///
  /// Allows users to select multiple files in the file picker.
  ///
  /// * [Chrome bug (for Android)](https://code.google.com/p/chromium/issues/detail?id=348912)
  /// * [Article](https://www.raymondcamden.com/2012/02/28/Working-with-HTML5s-multiple-file-upload-support)
  InputFileMultiple,
  /// inputmode attribute
  ///
  /// The `inputmode` attribute specifies what kind of input mechanism would be most helpful for users entering content into the form control.
  ///
  /// * [Demo on Wufoo (old)](https://www.wufoo.com/html5/attributes/23-inputmode.html)
  /// * [Everything You Ever Wanted to Know About inputmode (CSS Tricks)](https://css-tricks.com/everything-you-ever-wanted-to-know-about-inputmode/)
  InputInputmode,
  /// Minimum length attribute for input fields
  ///
  /// Declares a lower bound on the number of characters a user can input.
  ///
  /// * [W3C usage example](https://www.w3.org/TR/html5/forms.html#setting-minimum-input-length-requirements:-the-minlength-attribute)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=932755)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=149832)
  InputMinlength,
  /// Number input type
  ///
  /// Form field type for numbers.
  ///
  /// * [Polyfill](https://github.com/jonstipe/number-polyfill)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/form.js#input-type-number)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/input/type/number)
  /// * [Poor browser support for localized decimal marks, commas](https://www.ctrl.blog/entry/html5-input-number-localization.html)
  InputNumber,
  /// Pattern attribute for input fields
  ///
  /// Allows validation of an input field based on a given regular expression pattern.
  ///
  /// * [Site with common sample patterns](https://www.html5pattern.com/)
  /// * [MDN Web Docs - input element: pattern attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input#attr-pattern)
  InputPattern,
  /// input placeholder attribute
  ///
  /// Method of setting placeholder text for text-like input fields, to suggest the expected inserted information.
  ///
  /// * [Article on usage](https://www.zachleat.com/web/placeholder/)
  /// * [Polyfill](https://github.com/mathiasbynens/jquery-placeholder)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/form.js#input-attr-placeholder)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/attributes/placeholder)
  /// * [Issue 24626: Placeholder text for an input type=](https://code.google.com/p/android/issues/detail?id=24626)
  InputPlaceholder,
  /// Range input type
  ///
  /// Form field type that allows the user to select a value using a slider widget.
  ///
  /// * [Polyfill for Firefox](https://github.com/fryn/html5slider)
  /// * [Cross-browser polyfill](https://github.com/freqdec/fd-slider)
  /// * [Tutorial](http://tutorialzine.com/2011/12/what-you-need-to-know-html5-range-input/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/form.js#input-type-range)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/input/type/range)
  /// * [rangeslider.js polyfill](https://github.com/andreruffert/rangeslider.js)
  /// * [MDN web docs](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/range)
  /// * [Tutorial](https://tutorialzine.com/2011/12/what-you-need-to-know-html5-range-input)
  InputRange,
  /// Search input type
  ///
  /// Search field form input type. Intended to look like the underlying platform's native search field widget (if there is one). Other than its appearance, it's the same as an `<input type="text">`.
  ///
  /// * [CSS-Tricks article](https://css-tricks.com/webkit-html5-search-inputs/)
  /// * [Wufoo's The Current State of HTML5 Forms: The search Type](https://www.wufoo.com/html5/types/5-search.html)
  InputSearch,
  /// Selection controls for input & textarea
  ///
  /// Controls for setting and getting text selection via `setSelectionRange()` and the `selectionStart` & `selectionEnd` properties.
  ///
  /// * [MDN article on setSelectionRange](https://developer.mozilla.org/en-US/docs/Web/API/HTMLInputElement/setSelectionRange)
  InputSelection,
  /// Element.insertAdjacentElement() & Element.insertAdjacentText()
  ///
  /// Methods for inserting an element or text before or after a given element, or appending or prepending an element or text to a given element's list of children.
  ///
  /// * [WHATWG DOM Specification for Element.insertAdjacentText()](https://dom.spec.whatwg.org/#dom-element-insertadjacenttext)
  /// * [MDN Web Docs - Element.insertAdjacentElement()](https://developer.mozilla.org/en-US/docs/Web/API/Element/insertAdjacentElement)
  /// * [MDN Web Docs - Element.insertAdjacentText()](https://developer.mozilla.org/en-US/docs/Web/API/Element/insertAdjacentText)
  /// * [JS Bin testcase](https://jsbin.com/yanadu/edit?html,js,output)
  InsertAdjacent,
  /// Element.insertAdjacentHTML()
  ///
  /// Inserts a string of HTML into a specified position in the DOM relative to the given element.
  ///
  /// * [MDN Web Docs - insertAdjacentHTML](https://developer.mozilla.org/en-US/docs/Web/API/Element/insertAdjacentHTML)
  /// * [Polyfill](https://gist.github.com/eligrey/1276030)
  Insertadjacenthtml,
  /// Internationalization API
  ///
  /// Locale-sensitive collation (string comparison), number formatting, and date and time formatting.
  ///
  /// * [MDN Web Docs - Internationalization](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl)
  /// * [The ECMAScript Internationalization API](https://norbertlindenberg.com/2012/12/ecmascript-internationalization-api/)
  /// * [Working With Intl](https://code.tutsplus.com/tutorials/working-with-intl--cms-21082)
  /// * [WebKit tracking bug](https://bugs.webkit.org/show_bug.cgi?id=90906)
  /// * [Firefox for Android tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1344625)
  Internationalization,
  /// IntersectionObserver
  ///
  /// API that can be used to understand the visibility and position of DOM elements relative to a containing element or to the top-level viewport. The position is delivered asynchronously and is useful for understanding the visibility of elements and implementing pre-loading and deferred loading of DOM content.
  ///
  /// * [MDN Web Docs - Intersection Observer](https://developer.mozilla.org/en-US/docs/Web/API/Intersection_Observer_API)
  /// * [Polyfill](https://github.com/w3c/IntersectionObserver)
  /// * [Google Developers article](https://developers.google.com/web/updates/2016/04/intersectionobserver)
  Intersectionobserver,
  /// IntersectionObserver V2
  ///
  /// Iteration on the original API that also reports if the element is covered by another element or has filters applied to it. Useful for blocking clickjacking attempts or tracking ad exposure.
  ///
  /// * [Google Web Docs - Intersection Observer V2](https://developers.google.com/web/updates/2019/02/intersectionobserver-v2)
  /// * [Request for Mozilla Position on IntersectionObserver V2](https://github.com/mozilla/standards-positions/issues/109)
  /// * [Safari support bug](https://bugs.webkit.org/show_bug.cgi?id=251586)
  IntersectionobserverV2,
  /// Intl.PluralRules API
  ///
  /// API for plural sensitive formatting and plural language rules.
  ///
  /// * [MDN Web Docs: Intl.PluralRules](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/PluralRules)
  /// * [Google Developers blog: The Intl.PluralRules API](https://developers.google.com/web/updates/2017/10/intl-pluralrules)
  IntlPluralrules,
  /// Intrinsic & Extrinsic Sizing
  ///
  /// Allows for the heights and widths to be specified in intrinsic values using the `max-content`, `min-content`, `fit-content` and `stretch` (formerly `fill`) properties.
  ///
  /// * [Min-Content tutorial](https://thenewcode.com/662/Design-From-the-Inside-Out-With-CSS-Min-Content)
  IntrinsicWidth,
  /// JPEG 2000 image format
  ///
  /// JPEG 2000 was built to supersede the original JPEG format by having better compression and more features. [WebP](/webp), [AVIF](/avif) and [JPEG XL](/jpegxl) are all designed to supersede JPEG 2000.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/JPEG_2000)
  Jpeg2000,
  /// JPEG XL image format
  ///
  /// A modern image format optimized for web environments. JPEG XL generally has better compression than [WebP](/webp), JPEG, PNG and GIF and is designed to supersede them. JPEG XL competes with [AVIF](/avif) which has similar compression quality but fewer features overall.
  ///
  /// * [Official website](https://jpeg.org/jpegxl/index.html)
  /// * [Comparison to other formats](https://cloudinary.com/blog/how_jpeg_xl_compares_to_other_image_codecs)
  /// * [Chromium support bug](https://crbug.com/1178058)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1539075)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=208235)
  /// * [Request for Mozilla position on JPEG XL](https://github.com/mozilla/standards-positions/issues/522)
  /// * [2024 update on the request for Mozilla position on JPEG XL](https://github.com/mozilla/standards-positions/pull/1064)
  Jpegxl,
  /// JPEG XR image format
  ///
  /// JPEG XR was built to supersede the original JPEG format by having better compression and more features. [WebP](/webp), [AVIF](/avif) and [JPEG XL](/jpegxl) are all designed to supersede JPEG XR.
  ///
  /// * [Microsoft JPEG XR Codec Overview](https://docs.microsoft.com/en-us/windows/win32/wic/jpeg-xr-codec)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=500500)
  /// * [Chrome support bug (marked as WONTFIX)](https://code.google.com/p/chromium/issues/detail?id=56908)
  Jpegxr,
  /// Lookbehind in JS regular expressions
  ///
  /// The positive lookbehind (`(?<= )`) and negative lookbehind (`(?<! )`) zero-width assertions in JavaScript regular expressions can be used to ensure a pattern is preceded by another pattern.
  ///
  /// * [Blog post on lookbehind assertions](https://2ality.com/2017/05/regexp-lookbehind-assertions.html)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1225665)
  /// * [MDN: Regular Expressions Assertions](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_Expressions/Assertions)
  /// * [Safari implementation bug](https://bugs.webkit.org/show_bug.cgi?id=174931)
  JsRegexpLookbehind,
  /// JSON parsing
  ///
  /// Method of converting JavaScript objects to JSON strings and JSON back to objects using JSON.stringify() and JSON.parse()
  ///
  /// * [MDN Web Docs - Working with JSON](https://developer.mozilla.org/en-US/docs/Learn/JavaScript/Objects/JSON)
  /// * [JSON in JS (includes script w/support)](https://www.json.org/json-en.html)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/json.js#json)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/json)
  /// * [JSON explainer](https://www.json.org/)
  Json,
  /// CSS justify-content: space-evenly
  ///
  /// The "space-evenly" value for the `justify-content` property distributes the space between items evenly. It is similar to space-around but provides equal instead of half-sized space on the edges. Can be used in both CSS flexbox & grid.
  ///
  /// * [MDN on justify-content property](https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content)
  /// * [Edge support bug](https://web.archive.org/web/20190401105606/https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/15947692/)
  JustifyContentSpaceEvenly,
  /// High-quality kerning pairs & ligatures
  ///
  /// When used in HTML, the unofficial `text-rendering: optimizeLegibility` CSS property enables high-quality kerning and ligatures in certain browsers. Newer browsers have this behavior enabled by default.
  ///
  /// * [MDN Web Docs - CSS text-rendering](https://developer.mozilla.org/en-US/docs/Web/CSS/text-rendering)
  /// * [CSS Tricks article](https://css-tricks.com/almanac/properties/t/text-rendering/)
  KerningPairsLigatures,
  /// KeyboardEvent.charCode
  ///
  /// A legacy `KeyboardEvent` property that gives the Unicode codepoint number of a character key pressed during a `keypress` event.
  ///
  /// * [MDN Web Docs - charCode](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/charCode)
  KeyboardeventCharcode,
  /// KeyboardEvent.code
  ///
  /// A `KeyboardEvent` property representing the physical key that was pressed, ignoring the keyboard layout and ignoring whether any modifier keys were active.
  ///
  /// * [MDN Web Docs - code](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code)
  /// * [Chrome tracking bug](https://code.google.com/p/chromium/issues/detail?id=227231)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=149584)
  KeyboardeventCode,
  /// KeyboardEvent.getModifierState()
  ///
  /// `KeyboardEvent` method that returns the state (whether the key is pressed/locked or not) of the given modifier key.
  ///
  /// * [MDN Web Docs - getModifierState](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/getModifierState)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=40999)
  KeyboardeventGetmodifierstate,
  /// KeyboardEvent.key
  ///
  /// A `KeyboardEvent` property whose value is a string identifying the key that was pressed. Covers character keys, non-character keys (e.g. arrow keys), and dead keys.
  ///
  /// * [MDN Web Docs - key](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key)
  /// * [Chrome tracking bug](https://code.google.com/p/chromium/issues/detail?id=227231)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=69029)
  /// * [Spec listing all key string values](https://www.w3.org/TR/DOM-Level-3-Events-key/)
  /// * [Edge bug report](https://web.archive.org/web/20190401104951/https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/8860571/)
  /// * [shim-keyboard-event-key: shim for non-standard key identifiers for IE & Edge](https://github.com/shvaikalesh/shim-keyboard-event-key)
  KeyboardeventKey,
  /// KeyboardEvent.location
  ///
  /// A `KeyboardEvent` property that indicates the location of the key on the input device. Useful when there are more than one physical key for the same logical key (e.g. left or right "Control" key; main or numpad "1" key).
  ///
  /// * [MDN Web Docs - location](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/location)
  KeyboardeventLocation,
  /// KeyboardEvent.which
  ///
  /// A legacy `KeyboardEvent` property that is equivalent to either `KeyboardEvent.keyCode` or `KeyboardEvent.charCode` depending on whether the key is alphanumeric.
  ///
  /// * [MDN Web Docs - which](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/which)
  KeyboardeventWhich,
  /// Resource Hints: Lazyload
  ///
  /// Gives a hint to the browser to lower the loading priority of a resource. Please note that this is a legacy attribute, see the [`loading`](/loading-lazy-attr) attribute for the new standardized API.
  ///
  /// * [lazyload attribute | lazyload property](https://msdn.microsoft.com/en-us/ie/dn369270(v=vs.94))
  /// * [Discussion on standardization](https://github.com/whatwg/html/issues/2806)
  Lazyload,
  /// let
  ///
  /// Declares a variable with block level scope
  ///
  /// * [Variables and Constants in ES6](https://generatedcontent.org/post/54444832868/variables-and-constants-in-es6)
  /// * [MDN Web Docs - let](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/let)
  Let,
  /// PNG favicons
  ///
  /// Icon used by browsers to identify a webpage or site. While all browsers support the `.ico` format, the PNG format can be preferable.
  ///
  /// * [How to favicon in 2021](https://dev.to/masakudamatsu/favicon-nightmare-how-to-maintain-sanity-3al7)
  LinkIconPng,
  /// SVG favicons
  ///
  /// Icon used by browsers to identify a webpage or site. While all browsers support the `.ico` format, the SVG format can be preferable to more easily support higher resolutions or larger icons.
  ///
  /// * [Chrome bug](https://bugs.chromium.org/p/chromium/issues/detail?id=294179)
  /// * [Firefox bug, highlights comment that confirms note #4](https://bugzilla.mozilla.org/show_bug.cgi?id=366324#c50)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=136059)
  /// * [How to favicon in 2021](https://dev.to/masakudamatsu/favicon-nightmare-how-to-maintain-sanity-3al7)
  LinkIconSvg,
  /// Resource Hints: dns-prefetch
  ///
  /// Gives a hint to the browser to perform a DNS lookup in the background to improve performance. This is indicated using `<link rel="dns-prefetch" href="https://example.com/">`
  ///
  /// * [Prerender and prefetch support](https://msdn.microsoft.com/en-us/library/dn265039(v=vs.85).aspx)
  /// * [Controlling DNS prefetching](https://developer.mozilla.org/en-US/docs/Web/HTTP/Controlling_DNS_prefetching)
  /// * [What to <link rel=dns-prefetch> and when to use `preconnect` instead](https://www.ctrl.blog/entry/dns-prefetch-preconnect.html)
  LinkRelDnsPrefetch,
  /// Resource Hints: modulepreload
  ///
  /// Using `<link rel="modulepreload">`, browsers can be informed to prefetch module scripts without having to execute them, allowing fine-grained control over when and how module resources are loaded.
  ///
  /// * [Gecko implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1425310)
  /// * [WebKit implementation bug](https://bugs.webkit.org/show_bug.cgi?id=180574)
  /// * [Preloading modules](https://developers.google.com/web/updates/2017/12/modulepreload)
  /// * [Modern Script Loading](https://jasonformat.com/modern-script-loading/)
  LinkRelModulepreload,
  /// Resource Hints: preconnect
  ///
  /// Gives a hint to the browser to begin the connection handshake (DNS, TCP, TLS) in the background to improve performance. This is indicated using `<link rel="preconnect" href="https://example-domain.com/">`
  ///
  /// * [Eliminating Roundtrips with Preconnect](https://www.igvita.com/2015/08/17/eliminating-roundtrips-with-preconnect/)
  LinkRelPreconnect,
  /// Resource Hints: prefetch
  ///
  /// Informs the browsers that a given resource should be prefetched so it can be loaded more quickly. This is indicated using `<link rel="prefetch" href="(url)">`
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Link_prefetching)
  /// * [Article on prefetch and other hints](https://medium.com/@luisvieira_gmr/html5-prefetch-1e54f6dda15d)
  LinkRelPrefetch,
  /// Resource Hints: preload
  ///
  /// Using `<link rel="preload">`, browsers can be informed to prefetch resources without having to execute them, allowing fine-grained control over when and how resources are loaded. Only the following `as` values are supported: fetch, image, font, script, style, track.
  ///
  /// * [Preload: What Is It Good For?](https://www.smashingmagazine.com/2016/02/preload-what-is-it-good-for/)
  /// * [MDN Web Docs - Preloading content with rel="preload"](https://developer.mozilla.org/en-US/docs/Web/HTML/Preloading_content)
  /// * [Firefox meta support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=Rel%3Dpreload)
  LinkRelPreload,
  /// Resource Hints: prerender
  ///
  /// Gives a hint to the browser to render the specified page in the background, speeding up page load if the user navigates to it. This is indicated using `<link rel="prerender" href="(url)">`
  ///
  /// * [Prerender and prefetch support](https://msdn.microsoft.com/en-us/library/dn265039(v=vs.85).aspx)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=730101)
  LinkRelPrerender,
  /// Lazy loading via attribute for images & iframes
  ///
  /// The `loading` attribute on images & iframes gives authors control over when the browser should start loading the resource.
  ///
  /// * [Blog post](https://addyosmani.com/blog/lazy-loading/)
  /// * [Explainer](https://github.com/scott-little/lazyload)
  /// * [WebKit support bug](https://webkit.org/b/196698)
  /// * [Firefox support bug for lazy loading iframes](https://bugzilla.mozilla.org/show_bug.cgi?id=1622090)
  /// * [Polyfill](https://github.com/mfranzke/loading-attribute-polyfill)
  LoadingLazyAttr,
  /// localeCompare()
  ///
  /// The `localeCompare()` method returns a number indicating whether a reference string comes before or after or is the same as the given string in sort order.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/localeCompare)
  Localecompare,
  /// Magnetometer
  ///
  /// Defines a concrete sensor interface to measure magnetic field in the X, Y and Z axis.
  ///
  /// * [Demo](https://intel.github.io/generic-sensor-demos/vr-button/build/bundled/)
  /// * [Article](https://developers.google.com/web/updates/2017/09/sensors-for-the-web)
  Magnetometer,
  /// matches() DOM method
  ///
  /// Method of testing whether or not a DOM element matches a given selector. Formerly known (and largely supported with prefix) as matchesSelector.
  ///
  /// * [MDN Web Docs - Element matches](https://developer.mozilla.org/en/docs/Web/API/Element/matches)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/HTMLElement/matches)
  Matchesselector,
  /// matchMedia
  ///
  /// API for finding out whether or not a media query applies to the document.
  ///
  /// * [matchMedia.js polyfill](https://github.com/paulirish/matchMedia.js/)
  /// * [MDN Web Docs - matchMedia](https://developer.mozilla.org/en/DOM/window.matchMedia)
  /// * [MDN Web Docs - Using matchMedia](https://developer.mozilla.org/en/CSS/Using_media_queries_from_code)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/media_queries/apis/matchMedia)
  Matchmedia,
  /// MathML
  ///
  /// Special tags that allow mathematical formulas and notations to be written on web pages.
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/MathML)
  /// * [Cross-browser support script](https://www.mathjax.org)
  /// * [MDN Web Docs - MathML](https://developer.mozilla.org/en-US/docs/Web/MathML)
  /// * [MathML torture test](https://fred-wang.github.io/MathFonts/mozilla_mathml_test/)
  /// * [MathML in Chromium Project Roadmap](https://mathml.igalia.com/project/)
  Mathml,
  /// maxlength attribute for input and textarea elements
  ///
  /// Declares an upper bound on the number of characters the user can input. Normally the UI ignores attempts by the user to type in additional characters beyond this limit.
  ///
  /// * [MDN Web Docs - attribute maxlength](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input#attr-maxlength)
  Maxlength,
  /// Media Fragments
  ///
  /// Allows only part of a resource to be shown, based on the fragment identifier in the URL. Currently support is primarily limited to video track ranges.
  ///
  /// * [Media fragments on MDN](https://developer.mozilla.org/de/docs/Web/HTML/Using_HTML5_audio_and_video#Specifying_playback_range)
  MediaFragments,
  /// Media Capture from DOM Elements API
  ///
  /// API to capture Real-Time video and audio from a DOM element, such as a `<video>`, `<audio>`, or `<canvas>` element via the `captureStream` method, in the form of a `MediaStream`
  ///
  /// * [MDN Web Docs - capture from <canvas>](https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/captureStream)
  /// * [MDN Web Docs - capture from <video>/<audio>](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/captureStream)
  /// * [Google Developers article](https://developers.google.com/web/updates/2016/10/capture-stream)
  MediacaptureFromelement,
  /// MediaRecorder API
  ///
  /// The MediaRecorder API (MediaStream Recording) aims to provide a really simple mechanism by which developers can record media streams from the user's input devices and instantly use them in web apps, rather than having to perform manual encoding operations on raw PCM data, etc.
  ///
  /// * [MDN Web Docs - MediaRecorder](https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder_API)
  Mediarecorder,
  /// Media Source Extensions
  ///
  /// API allowing media data to be accessed from HTML `video` and `audio` elements.
  ///
  /// * [MDN Web Docs - MediaSource](https://developer.mozilla.org/en-US/docs/Web/API/MediaSource)
  /// * [MSDN article](https://msdn.microsoft.com/en-us/library/dn594470%28v=vs.85%29.aspx)
  /// * [MediaSource demo](https://simpl.info/mse/)
  Mediasource,
  /// Context menu item (menuitem element)
  ///
  /// Method of defining a context menu item, now deprecated and [removed from the HTML specification](https://github.com/whatwg/html/issues/2730).
  ///
  /// * [Demo](https://bug617528.bugzilla.mozilla.org/attachment.cgi?id=554309)
  /// * [jQuery polyfill](https://github.com/swisnl/jQuery-contextMenu)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/events.js#event-contextmenu)
  /// * [Bug on Firefox support](https://bugzilla.mozilla.org/show_bug.cgi?id=746087)
  /// * [Chromium support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=87553)
  Menu,
  /// theme-color Meta Tag
  ///
  /// Meta tag to define a suggested color that browsers should use to customize the display of the page or of the surrounding user interface. The meta tag overrides any theme-color set in the web app manifest.
  ///
  /// * [Firefox for Android implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1098544)
  /// * [Google Developers article](https://developers.google.com/web/updates/2014/11/Support-for-theme-color-in-Chrome-39-for-Android?hl=en)
  MetaThemeColor,
  /// meter element
  ///
  /// Method of indicating the current level of a gauge.
  ///
  /// * [MDN Web Docs - Element meter](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
  /// * [HTML5 Doctor on meter element](https://html5doctor.com/measure-up-with-the-meter-tag/)
  /// * [Dev.Opera article](https://dev.opera.com/articles/new-form-features-in-html5/#newoutput)
  /// * [Examples of progress and meter elements](https://peter.sh/examples/?/html/meter-progress.html)
  /// * [The HTML `<meter>` element and its (undefined) segment boundaries](https://www.ctrl.blog/entry/html-meter-segment-boundaries.html)
  Meter,
  /// Web MIDI API
  ///
  /// The Web MIDI API specification defines a means for web developers to enumerate, manipulate and access MIDI devices
  ///
  /// * [Polyfill](https://github.com/cwilso/WebMIDIAPIShim)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=836897)
  /// * [Test/demo page](https://www.onlinemusictools.com/webmiditest/)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=107250)
  Midi,
  /// CSS min/max-width/height
  ///
  /// Method of setting a minimum or maximum width or height to an element.
  ///
  /// * [JS library with support](https://code.google.com/archive/p/ie7-js/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/min-width)
  /// * [CSS Basics post](https://www.impressivewebs.com/min-max-width-height-css/)
  Minmaxwh,
  /// MP3 audio format
  ///
  /// Popular lossy audio compression format
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/MP3)
  Mp3,
  /// Dynamic Adaptive Streaming over HTTP (MPEG-DASH)
  ///
  /// HTTP-based media streaming communications protocol, an alternative to HTTP Live Streaming (HLS).
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Dynamic_Adaptive_Streaming_over_HTTP)
  /// * [JavaScript implementation](https://github.com/Dash-Industry-Forum/dash.js/)
  MpegDash,
  /// MPEG-4/H.264 video format
  ///
  /// Commonly used video compression format.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/H.264/MPEG-4_AVC)
  /// * [Firefox extension allowing support in Win7](http://www.interoperabilitybridges.com/html5-extension-for-wmp-plugin)
  Mpeg4,
  /// CSS3 Multiple backgrounds
  ///
  /// Method of using multiple images as a background
  ///
  /// * [Demo & information page](https://www.css3.info/preview/multiple-backgrounds/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/background-image)
  Multibackgrounds,
  /// CSS3 Multiple column layout
  ///
  /// Method of flowing information in multiple columns
  ///
  /// * [Dev.Opera article](https://dev.opera.com/articles/view/css3-multi-column-layout/)
  /// * [Introduction page](https://webdesign.tutsplus.com/articles/an-introduction-to-the-css3-multiple-column-layout-module--webdesign-4934)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/column-width)
  /// * [Polyfill](https://github.com/hamsterbacke23/multicolumn-polyfill)
  /// * [Chrome platform status for CSS column-fill](https://www.chromestatus.com/feature/6298909664083968)
  Multicolumn,
  /// Mutation events
  ///
  /// Deprecated mechanism for listening to changes made to the DOM, replaced by Mutation Observers.
  ///
  /// * [MDN Web Docs - Mutation events](https://developer.mozilla.org/en-US/docs/Web/Guide/Events/Mutation_events)
  MutationEvents,
  /// Mutation Observer
  ///
  /// Method for observing and reacting to changes to the DOM. Replaces MutationEvents, which is deprecated.
  ///
  /// * [MutationObserver from MDN](https://developer.mozilla.org/en-US/docs/Web/API/MutationObserver)
  /// * [Polyfill](https://github.com/webcomponents/webcomponentsjs)
  Mutationobserver,
  /// Web Storage - name/value pairs
  ///
  /// Method of storing data locally like cookies, but for larger amounts of data (sessionStorage and localStorage, used to fall under HTML5).
  ///
  /// * [MDN Web Docs - Web Storage](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)
  /// * [Support library](https://code.google.com/archive/p/sessionstorage/downloads)
  /// * [Simple demo](https://html5demos.com/storage)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-localstorage;native-sessionstorage)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/web-storage/Storage/localStorage)
  NamevalueStorage,
  /// File System Access API
  ///
  /// API for manipulating files in the device's local file system (not in a sandbox).
  ///
  /// * [Explainer](https://github.com/WICG/file-system-access/blob/master/EXPLAINER.md)
  /// * [Web.dev blog post](https://web.dev/file-system-access/)
  /// * [Firefox position: harmful](https://mozilla.github.io/standards-positions/#file-system-access)
  /// * [Chrome blog post](https://developers.google.com/web/updates/2019/08/native-file-system)
  NativeFilesystemApi,
  /// Navigation Timing API
  ///
  /// API for accessing timing information related to navigation and elements.
  ///
  /// * [MDN Web Docs - Navigation Timing](https://developer.mozilla.org/en-US/docs/Web/API/Navigation_timing_API)
  /// * [HTML5 Rocks tutorial](https://www.html5rocks.com/en/tutorials/webperformance/basics/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/navigation_timing)
  NavTiming,
  /// Network Information API
  ///
  /// The Network Information API enables web applications to access information about the network connection in use by the device. Accessed via `navigator.connection`
  ///
  /// * [(NetInfo) Capability reporting with ServiceWorker](https://www.igvita.com/2014/12/15/capability-reporting-with-service-worker/)
  Netinfo,
  /// Web Notifications
  ///
  /// Method of alerting the user outside of a web page by displaying notifications (that do not require interaction by the user).
  ///
  /// * [HTML5 Rocks tutorial](https://www.html5rocks.com/tutorials/notifications/quick/)
  /// * [Chromium API](https://www.chromium.org/developers/design-documents/desktop-notifications/api-specification/)
  /// * [Add-on](https://addons.mozilla.org/en-us/firefox/addon/221523/)
  /// * [MDN Web Docs - Notification](https://developer.mozilla.org/en-US/docs/Web/API/notification)
  /// * [SitePoint article](https://www.sitepoint.com/introduction-web-notifications-api/)
  /// * [Demo](https://audero.it/demo/web-notifications-api-demo.html)
  /// * [Plug-in for support in IE](https://ie-web-notifications.github.io/)
  Notifications,
  /// Object.entries
  ///
  /// The `Object.entries()` method creates a multi-dimensional array of key value pairs from the given object.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/entries)
  /// * [ES2017 spec-compliant shim for Object.entries](https://github.com/es-shims/Object.entries)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-object)
  ObjectEntries,
  /// CSS3 object-fit/object-position
  ///
  /// Method of specifying how an object (image or video) should fit inside its box. object-fit options include "contain" (fit according to aspect ratio), "fill" (stretches object to fill) and "cover" (overflows box but maintains ratio), where object-position allows the object to be repositioned like background-image does.
  ///
  /// * [Dev.Opera article](https://dev.opera.com/articles/view/css3-object-fit-object-position/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/object-fit)
  /// * [object-fit-images Polyfill for IE & Edge](https://github.com/bfred-it/object-fit-images/)
  /// * [MDN (object-fit)](https://developer.mozilla.org/docs/Web/CSS/object-fit)
  /// * [MDN (object-position)](https://developer.mozilla.org/docs/Web/CSS/object-position)
  ObjectFit,
  /// Object.observe data binding
  ///
  /// Method for data binding, a now-withdrawn ECMAScript 7 proposal
  ///
  /// * [Data-binding Revolutions with Object.observe()](https://www.html5rocks.com/en/tutorials/es7/observe/)
  /// * [Polyfill](https://github.com/MaxArt2501/object-observe)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=800355)
  /// * [An update on Object.observe](https://esdiscuss.org/topic/an-update-on-object-observe)
  ObjectObserve,
  /// Object.values method
  ///
  /// The `Object.values()` method returns an array of a given object's own enumerable property values.
  ///
  /// * [Object.values() on MDN Web Docs](https://developer.mozilla.org/en/docs/Web/JavaScript/Reference/Global_objects/Object/values)
  /// * [Polyfill](https://github.com/es-shims/Object.values)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-object)
  ObjectValues,
  /// Object RTC (ORTC) API for WebRTC
  ///
  /// Enables mobile endpoints to talk to servers and web browsers with Real-Time Communications (RTC) capabilities via native and simple JavaScript APIs
  ///
  /// * [Related blog posts by Microsoft](https://blogs.windows.com/msedgedev/tag/object-rtc/)
  Objectrtc,
  /// Offline web applications
  ///
  /// Now deprecated method of defining web page files to be cached using a cache manifest file, allowing them to work offline on subsequent visits to the page.
  ///
  /// * [Sitepoint tutorial](https://www.sitepoint.com/offline-web-application-tutorial/)
  /// * [Dive Into HTML5 article](http://diveintohtml5.info/offline.html)
  /// * [Mozilla Hacks article/demo](https://hacks.mozilla.org/2010/01/offline-web-applications/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/appcache/ApplicationCache)
  OfflineApps,
  /// OffscreenCanvas
  ///
  /// OffscreenCanvas allows canvas drawing to occur with no connection to the DOM and can be used inside workers.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/API/OffscreenCanvas)
  /// * [WebGL off the main thread - Mozilla Hacks article](https://hacks.mozilla.org/2016/01/webgl-off-the-main-thread/)
  /// * [Making the whole web better, one canvas at a time. - Article about canvas performance](https://bkardell.com/blog/OffscreenCanvas.html)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1390089)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=183720)
  Offscreencanvas,
  /// Ogg Vorbis audio format
  ///
  /// Vorbis is a free and open source audio format, most commonly used with the Ogg container.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Vorbis)
  OggVorbis,
  /// Ogg/Theora video format
  ///
  /// Free lossy video compression format.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Theora)
  /// * [Chrome Platform Status: Deprecate and remove Theora support](https://chromestatus.com/feature/5158654475239424)
  /// * [Firefox bug: Investigate removing Theora support](https://bugzilla.mozilla.org/show_bug.cgi?id=1860492)
  Ogv,
  /// Reversed attribute of ordered lists
  ///
  /// This attribute makes an ordered list number its items in descending order (large to small), instead of ascending order (small to large; the default). The order that the list items are displayed in is not affected.
  ///
  /// * [HTML5 Doctor article on <ol> element attributes (including reversed)](https://html5doctor.com/ol-element-attributes/)
  OlReversed,
  /// "once" event listener option
  ///
  /// Causes an event listener to be automatically removed after it gets invoked, so that it only gets invoked once. Similar to jQuery's `$.one()` feature.
  ///
  /// * [Chromium Issue 615384: Support "once" event listener option](https://bugs.chromium.org/p/chromium/issues/detail?id=615384)
  /// * [JS Bin testcase](https://jsbin.com/zigiru/edit?html,js,output)
  OnceEventListener,
  /// Online/offline status
  ///
  /// Events to indicate when the user's connected (`online` and `offline` events) and the `navigator.onLine` property to see current status.
  ///
  /// * [MDN Web Docs - NavigatorOnLine.onLine](https://developer.mozilla.org/en-US/docs/Web/API/NavigatorOnLine.onLine#Specification)
  OnlineStatus,
  /// Opus audio format
  ///
  /// Royalty-free open audio codec by IETF, which incorporated SILK from Skype and CELT from Xiph.org, to serve higher sound quality and lower latency at the same bitrate.
  ///
  /// * [Introduction of Opus by Mozilla](https://hacks.mozilla.org/2012/07/firefox-beta-15-supports-the-new-opus-audio-format/)
  /// * [Google's statement about the use of VP8 and Opus codec for WebRTC standard](https://www.ietf.org/mail-archive/web/rtcweb/current/msg04953.html)
  Opus,
  /// Orientation Sensor
  ///
  /// Defines a base orientation sensor interface and concrete sensor subclasses to monitor the device’s physical orientation in relation to a stationary three dimensional Cartesian coordinate system.
  ///
  /// * [Demo](https://intel.github.io/generic-sensor-demos/orientation-phone/)
  /// * [Article](https://developers.google.com/web/updates/2017/09/sensors-for-the-web#orientation-sensors)
  OrientationSensor,
  /// CSS outline properties
  ///
  /// The CSS outline properties draw a border around an element that does not affect layout, making it ideal for highlighting. This covers the `outline` shorthand, as well as `outline-width`, `outline-style`, `outline-color` and `outline-offset`.
  ///
  /// * [CSS Basic User Interface Module Level 3](https://drafts.csswg.org/css-ui-3/#outline)
  /// * [MDN Web Docs - CSS outline](https://developer.mozilla.org/en-US/docs/CSS/outline)
  Outline,
  /// String.prototype.padStart(), String.prototype.padEnd()
  ///
  /// The `padStart()` and `padEnd()` methods pad the current string with a given string (eventually repeated) so that the resulting string reaches a given length. The pad is applied from the start (left) of the current string for `padStart()`, and applied from the end (right) of the current string for `padEnd()`.
  ///
  /// * [MDN Web Docs - padStart()](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/padStart)
  /// * [MDN Web Docs - padEnd()](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/padEnd)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-string-and-regexp)
  PadStartEnd,
  /// PageTransitionEvent
  ///
  /// Fired at the Window when the page's entry in the session history stops being the current entry. Includes the `pageshow` and `pagehide` events.
  ///
  /// * [MDN Web Docs - pageshow](https://developer.mozilla.org/en-US/docs/Web/Events/pageshow)
  /// * [HTML onpageshow Event Attribute](https://www.w3schools.com/tags/ev_onpageshow.asp)
  /// * [Back/forward cache: web-exposed behaviour](https://docs.google.com/document/d/1JtDCN9A_1UBlDuwkjn1HWxdhQ1H2un9K4kyPLgBqJUc)
  /// * [Back-forward cache on Android](https://docs.google.com/document/d/1E7LY4HxkJxIjNt9PJIq5vKtNh6hB0PCTzENGkoYAbgA)
  /// * [web.dev - Back/forward cache](https://web.dev/bfcache/)
  PageTransitionEvents,
  /// Page Visibility
  ///
  /// JavaScript API for determining whether a document is visible on the display
  ///
  /// * [MDN Web Docs - Page Visibility](https://developer.mozilla.org/en-US/docs/DOM/Using_the_Page_Visibility_API)
  /// * [SitePoint article](https://www.sitepoint.com/introduction-to-page-visibility-api/)
  /// * [Demo](https://audero.it/demo/page-visibility-api-demo.html)
  Pagevisibility,
  /// Passive event listeners
  ///
  /// Event listeners created with the `passive: true` option cannot cancel (`preventDefault()`) the events they receive. Primarily intended to be used with touch events and `wheel` events. Since they cannot prevent scrolls, passive event listeners allow the browser to perform optimizations that result in smoother scrolling.
  ///
  /// * [Improving scroll performance with passive event listeners - Google Developers Updates](https://developers.google.com/web/updates/2016/06/passive-event-listeners?hl=en)
  /// * [Polyfill from the WICG](https://github.com/WICG/EventListenerOptions/blob/gh-pages/EventListenerOptions.polyfill.js)
  /// * [Original WICG EventListenerOptions repository](https://github.com/WICG/EventListenerOptions)
  /// * [JS Bin testcase](https://jsbin.com/jaqaku/edit?html,js,output)
  PassiveEventListener,
  /// Passkeys
  ///
  /// Passkeys, also known as Multi-device FIDO Credentials, provide users with an alternative to passwords that is much easier to use and far more secure.
  ///
  /// * [Passkeys at apple.com](https://developer.apple.com/passkeys/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1792433)
  /// * [Google article on Passkeys for developers](https://developers.google.com/identity/passkeys)
  /// * [FIDO alliance article on Passkeys](https://fidoalliance.org/passkeys/)
  Passkeys,
  /// Path2D
  ///
  /// Allows path objects to be declared on 2D canvas surfaces
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/API/Path2D)
  Path2d,
  /// Payment Request API
  ///
  /// Payment Request is a new API for the open web that makes checkout flows easier, faster and consistent on shopping sites.
  ///
  /// * [Spec discussion](https://github.com/w3c/browser-payment-api/)
  /// * [Bringing easy and fast checkout with Payment Request API](https://developers.google.com/web/updates/2016/07/payment-request)
  /// * [Payment Request API Integration Guide](https://developers.google.com/web/fundamentals/discovery-and-monetization/payment-request/)
  /// * [MDN Web Docs - Payment Request API](https://developer.mozilla.org/en-US/docs/Web/API/Payment_Request_API)
  /// * [Demo](https://paymentrequest.show/demo)
  /// * [Simpler Demos and Codes](https://googlechrome.github.io/samples/paymentrequest/)
  PaymentRequest,
  /// Built-in PDF viewer
  ///
  /// Support for a PDF viewer that is part of the browser, rather than requiring a PDF file to be opened in an external application.
  ///
  /// * [PDFObject - JavaScript utility to embed PDF documents in HTML](https://pdfobject.com)
  PdfViewer,
  /// Permissions API
  ///
  /// High-level JavaScript API for checking and requesting permissions
  ///
  /// * [Permission API samples and examples](https://developer.chrome.com/blog/permissions-api-for-the-web/)
  /// * [Extended "polyfill" version of permission API](https://github.com/jimmywarting/browser-su)
  PermissionsApi,
  /// Permissions Policy
  ///
  /// A security mechanism that allows developers to explicitly enable or disable various powerful browser features for a given site. Similar to [Document Policy](/document-policy).
  ///
  /// * [W3C - Permissions Policy Explainer](https://github.com/w3c/webappsec-feature-policy/blob/main/permissions-policy-explainer.md)
  /// * [Firefox implementation tracker](https://bugzilla.mozilla.org/show_bug.cgi?id=1531012)
  /// * [List of known features](https://github.com/w3c/webappsec-permissions-policy/blob/main/features.md)
  PermissionsPolicy,
  /// Picture element
  ///
  /// A responsive images method to control which image resource a user agent presents to a user, based on resolution, media query and/or support for a particular image format
  ///
  /// * [Demo](https://responsiveimages.org/demos/)
  /// * [Tutorial](https://code.tutsplus.com/tutorials/better-responsive-images-with-the-picture-element--net-36583)
  /// * [Read about the use cases](https://usecases.responsiveimages.org/)
  /// * [General information about Responsive Images](https://responsiveimages.org/)
  /// * [Blog post on usage](https://dev.opera.com/articles/responsive-images/)
  /// * [HTML5 Rocks tutorial](https://www.html5rocks.com/tutorials/responsive/picture-element/)
  /// * [Picturefill - polyfill for picture, srcset, sizes, and more](https://github.com/scottjehl/picturefill)
  Picture,
  /// Picture-in-Picture
  ///
  /// Allows websites to create a floating video window that is always on top of other windows so that users may continue consuming media while they interact with other sites or applications on their device.
  ///
  /// * [Sample video](https://googlechrome.github.io/samples/picture-in-picture/)
  /// * [Safari equivalent API](https://developer.apple.com/documentation/webkitjs/adding_picture_in_picture_to_your_safari_media_controls)
  /// * [Opera equivalent Video Pop Out feature](https://blogs.opera.com/desktop/2016/04/opera-beta-update-video-pop/)
  /// * [Implementation Status](https://github.com/WICG/picture-in-picture/blob/master/implementation-status.md)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1519885)
  PictureInPicture,
  /// Ping attribute
  ///
  /// When used on an anchor, this attribute signifies that the browser should send a ping request the resource the attribute points to.
  ///
  /// * [MDN Web Docs - Element ping attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a#attr-ping)
  Ping,
  /// PNG alpha transparency
  ///
  /// Semi-transparent areas in PNG files
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/Portable_Network_Graphics)
  PngAlpha,
  /// Pointer events
  ///
  /// This specification integrates various inputs from mice, touchscreens, and pens, making separate implementations no longer necessary and authoring for cross-device pointers easier. Not to be mistaken with the unrelated "pointer-events" CSS property.
  ///
  /// * [Abstraction library for pointer events](https://deeptissuejs.com/)
  /// * [PEP: Pointer Events Polyfill](https://github.com/jquery/PEP)
  /// * [Pointer Event API on MDN](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events)
  /// * [Bugzilla@Mozilla: Bug 822898 - Implement pointer events](https://bugzilla.mozilla.org/show_bug.cgi?id=822898)
  Pointer,
  /// CSS pointer-events (for HTML)
  ///
  /// This CSS property, when set to "none" allows elements to not receive hover/click events, instead the event will occur on anything behind it.
  ///
  /// * [Article & tutorial](https://robertnyman.com/2010/03/22/css-pointer-events-to-allow-clicks-on-underlying-elements/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/css.js#css-pointerevents)
  /// * [Polyfill](https://github.com/kmewhort/pointer_events_polyfill)
  PointerEvents,
  /// Pointer Lock API
  ///
  /// API that provides access to raw mouse movement data. This is done by ignoring boundaries resulting from screen edges where the cursor can't go beyond, providing proper control for first person or real time strategy games.
  ///
  /// * [MDN Web Docs - Pointer Lock](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_Lock_API)
  /// * [Simple demo](https://mdn.github.io/dom-examples/pointer-lock/)
  Pointerlock,
  /// Portals
  ///
  /// Portals enable seamless navigation between sites or pages. A new page can be loaded as an inset using the `<portal>` element (similar to an iframe) which can then seamlessly transition to the new navigated state when "activated".
  ///
  /// * [Hands-on with Portals: seamless navigation on the Web](https://web.dev/hands-on-portals/)
  Portals,
  /// prefers-color-scheme media query
  ///
  /// Media query to detect if the user has set their system to use a light or dark color theme.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-color-scheme)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1494034)
  /// * [Chromium implementation issue](https://bugs.chromium.org/p/chromium/issues/detail?id=889087)
  /// * [Web.dev article](https://web.dev/prefers-color-scheme/)
  PrefersColorScheme,
  /// prefers-reduced-motion media query
  ///
  /// CSS media query based on a user preference for preferring reduced motion (animation, etc).
  ///
  /// * [WebKit blog post](https://webkit.org/blog/7551/responsive-design-for-motion/)
  /// * [CSS Tricks article](https://css-tricks.com/introduction-reduced-motion-media-query/)
  PrefersReducedMotion,
  /// progress element
  ///
  /// Method of indicating a progress state.
  ///
  /// * [CSS-Tricks article](https://css-tricks.com/html5-progress-element/)
  /// * [MDN Web Docs - Element progress](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
  /// * [Dev.Opera article](https://dev.opera.com/articles/new-form-features-in-html5/#newoutput)
  /// * [Examples of progress and meter elements](https://peter.sh/examples/?/html/meter-progress.html)
  Progress,
  /// Promise.prototype.finally
  ///
  /// When the promise is settled, whether fulfilled or rejected, the specified callback function is executed.
  ///
  /// * [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise/finally)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-promise)
  PromiseFinally,
  /// Promises
  ///
  /// A promise represents the eventual result of an asynchronous operation.
  ///
  /// * [Promises/A+ spec](https://promisesaplus.com/)
  /// * [JavaScript Promises: There and back again - HTML5 Rocks](https://www.html5rocks.com/en/tutorials/es6/promises/)
  /// * [A polyfill for ES6-style Promises](https://github.com/jakearchibald/ES6-Promises)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-promise)
  Promises,
  /// Proximity API
  ///
  /// Defines events that provide information about the distance between a device and an object, as measured by a proximity sensor.
  ///
  /// * [Demo](https://audero.it/demo/proximity-api-demo.html)
  /// * [SitePoint article](https://www.sitepoint.com/introducing-proximity-api/)
  Proximity,
  /// Proxy object
  ///
  /// The Proxy object allows custom behavior to be defined for fundamental operations. Useful for logging, profiling, object visualization, etc.
  ///
  /// * [ECMAScript 6 Proxies](https://github.com/lukehoban/es6features#proxies)
  /// * [MDN Web Docs - Proxy](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Proxy)
  /// * [Experimenting with ECMAScript 6 proxies](https://humanwhocodes.com/blog/2011/09/15/experimenting-with-ecmascript-6-proxies/)
  /// * [Meta programming with ECMAScript 6 proxies](https://2ality.com/2014/12/es6-proxies.html)
  /// * [Polyfill for Proxies](https://github.com/tvcutsem/harmony-reflect)
  Proxy,
  /// HTTP Public Key Pinning
  ///
  /// Declare that a website's HTTPS certificate should only be treated as valid if the public key is contained in a list specified over HTTP to prevent MITM attacks that use valid CA-issued certificates.
  ///
  /// * [MDN Web Docs - Public Key Pinning](https://developer.mozilla.org/en-US/docs/Web/Security/Public_Key_Pinning)
  /// * [Scott Helme article on the issues of HPKP](https://scotthelme.co.uk/im-giving-up-on-hpkp/)
  Publickeypinning,
  /// Push API
  ///
  /// API to allow messages to be pushed from a server to a browser, even when the site isn't focused or even open in the browser.
  ///
  /// * [MDN Web Docs - Push API](https://developer.mozilla.org/en-US/docs/Web/API/Push_API)
  /// * [Google Developers article](https://developers.google.com/web/updates/2015/03/push-notifications-on-the-open-web)
  PushApi,
  /// querySelector/querySelectorAll
  ///
  /// Method of accessing DOM elements using CSS selectors
  ///
  /// * [MDN Web Docs - querySelector](https://developer.mozilla.org/en-US/docs/Web/API/Element/querySelector)
  /// * [MDN Web Docs - querySelectorAll](https://developer.mozilla.org/en-US/docs/Web/API/Element/querySelectorAll)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/selectors_api/querySelector)
  Queryselector,
  /// readonly attribute of input and textarea elements
  ///
  /// Makes the form control non-editable. Unlike the `disabled` attribute, `readonly` form controls are still included in form submissions and the user can still select (but not edit) their value text.
  ///
  /// * [WHATWG HTML specification for the readonly attribute of the `<textarea>` element](https://html.spec.whatwg.org/multipage/forms.html#attr-textarea-readonly)
  /// * [MDN Web Docs - readonly attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/Input#attr-readonly)
  ReadonlyAttr,
  /// Referrer Policy
  ///
  /// A policy that controls how much information is shared through the HTTP `Referer` header. Helps to protect user privacy.
  ///
  /// * [Mozilla security article](https://blog.mozilla.org/security/2015/01/21/meta-referrer/)
  /// * [A new security header: Referrer Policy](https://scotthelme.co.uk/a-new-security-header-referrer-policy/)
  ReferrerPolicy,
  /// Custom protocol handling
  ///
  /// Method of allowing a webpage to handle a given protocol using `navigator.registerProtocolHandler`. This allows certain URLs to be opened by a given web application, for example `mailto:` addresses can be opened by a webmail client.
  ///
  /// * [MDN Web Docs - Register protocol handler](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/registerProtocolHandler)
  Registerprotocolhandler,
  /// rel=noopener
  ///
  /// Ensure new browsing contexts are opened without a useful `window.opener`
  ///
  /// * [Explainer](https://mathiasbynens.github.io/rel-noopener/)
  /// * [Gecko/Firefox issue](https://bugzilla.mozilla.org/show_bug.cgi?id=1222516)
  /// * [WebKit/Safari issue](https://bugs.webkit.org/show_bug.cgi?id=155166)
  RelNoopener,
  /// Link type "noreferrer"
  ///
  /// Links with `rel="noreferrer"` set do not send the request's "referrer" header. This prevents the destination site from seeing what URL the user came from.
  ///
  /// * [Blog post on rel="noreferrer"](https://www.lifewire.com/rel-noreferrer-3468002)
  RelNoreferrer,
  /// relList (DOMTokenList)
  ///
  /// Method of easily manipulating rel attribute values on elements, using the DOMTokenList object (similar to classList).
  ///
  /// * [MDN Web Docs - DOMTokenList](https://developer.mozilla.org/en-US/docs/DOM/DOMTokenList)
  /// * [domtokenlist polyfill](https://github.com/jwilsson/domtokenlist)
  Rellist,
  /// rem (root em) units
  ///
  /// Type of unit similar to `em`, but relative only to the root element, not any parent element. Thus compounding does not occur as it does with `em` units.
  ///
  /// * [Article on usage](https://snook.ca/archives/html_and_css/font-size-with-rem)
  /// * [REM Polyfill](https://github.com/chuckcarpenter/REM-unit-polyfill)
  Rem,
  /// requestAnimationFrame
  ///
  /// API allowing a more efficient way of running script-based animation, compared to traditional methods using timeouts. Also covers support for `cancelAnimationFrame`
  ///
  /// * [Blog post](https://www.paulirish.com/2011/requestanimationframe-for-smart-animating/)
  /// * [Mozilla Hacks article](https://hacks.mozilla.org/2011/08/animating-with-javascript-from-setinterval-to-requestanimationframe/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/Window/requestAnimationFrame)
  /// * [MDN Web Docs - requestAnimationFrame](https://developer.mozilla.org/en-US/docs/Web/API/window/requestAnimationFrame)
  Requestanimationframe,
  /// requestIdleCallback
  ///
  /// API allowing the execution of JavaScript to be queued to run in idle browser time, either at the end of a frame or when the user is inactive. Also covers support for `cancelIdleCallback`. The API has similarities with `requestAnimationFrame`.
  ///
  /// * [MDN Web Docs - requestIdleCallback](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestIdleCallback)
  /// * [Google Developers article](https://developers.google.com/web/updates/2015/08/using-requestidlecallback)
  /// * [Shim](https://gist.github.com/paullewis/55efe5d6f05434a96c36)
  Requestidlecallback,
  /// Resize Observer
  ///
  /// Method for observing and reacting to changes to sizes of DOM elements.
  ///
  /// * [Google Developers Article](https://developers.google.com/web/updates/2016/10/resizeobserver)
  /// * [Explainer Doc](https://github.com/WICG/ResizeObserver/blob/master/explainer.md)
  /// * [Polyfill based on initial specification](https://github.com/que-etc/resize-observer-polyfill)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1272409)
  /// * [WebKit implementation bug](https://bugs.webkit.org/show_bug.cgi?id=157743)
  /// * [Polyfill based on latest specification which includes support for observer options](https://github.com/juggle/resize-observer)
  Resizeobserver,
  /// Resource Timing (basic support)
  ///
  /// Method to help web developers to collect complete timing information related to resources on a document.
  ///
  /// * [Demo](https://www.audero.it/demo/resource-timing-api-demo.html)
  /// * [Blog post](https://developers.googleblog.com/2013/12/measuring-network-performance-with.html)
  /// * [SitePoint article](https://www.sitepoint.com/introduction-resource-timing-api/)
  ResourceTiming,
  /// Rest parameters
  ///
  /// Allows representation of an indefinite number of arguments as an array.
  ///
  /// * [Rest parameters and defaults](https://hacks.mozilla.org/2015/05/es6-in-depth-rest-parameters-and-defaults/)
  RestParameters,
  /// WebRTC Peer-to-peer connections
  ///
  /// Method of allowing two users to communicate directly, browser to browser using the RTCPeerConnection API.
  ///
  /// * [WebRTC Project site](https://webrtc.org/)
  /// * [Plug-in for support in IE & Safari](https://temasys.atlassian.net/wiki/display/TWPP/WebRTC+Plugins)
  /// * [Introducing WebRTC 1.0 and interoperable real-time communications in Microsoft Edge](https://blogs.windows.com/msedgedev/2017/01/31/introducing-webrtc-microsoft-edge/)
  Rtcpeerconnection,
  /// Ruby annotation
  ///
  /// Method of adding pronunciation or other annotations using ruby elements (primarily used in East Asian typography).
  ///
  /// * [HTML5 Doctor article](https://html5doctor.com/ruby-rt-rp-element/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/ruby)
  /// * [CSS specification](https://www.w3.org/TR/css-ruby-1/)
  Ruby,
  /// display: run-in
  ///
  /// If the run-in box contains a block box, same as block. If a block box follows the run-in box, the run-in box becomes the first inline box of the block box. If an inline box follows, the run-in box becomes a block box.
  ///
  /// * [Mozilla bug report](https://bugzilla.mozilla.org/show_bug.cgi?id=2056)
  /// * [CSS Tricks article](https://css-tricks.com/run-in/)
  RunIn,
  /// 'SameSite' cookie attribute
  ///
  /// Same-site cookies ("First-Party-Only" or "First-Party") allow servers to mitigate the risk of CSRF and information leakage attacks by asserting that a particular cookie should only be sent with requests initiated from the same registrable domain.
  ///
  /// * [Preventing CSRF with the same-site cookie attribute](https://www.sjoerdlangkemper.nl/2016/04/14/preventing-csrf-with-samesite-cookie-attribute/)
  /// * [Mozilla Bug #795346: Add SameSite support for cookies](https://bugzilla.mozilla.org/show_bug.cgi?id=795346)
  /// * [Mozilla Bug #1286861, includes the patches that landed SameSite support in Firefox](https://bugzilla.mozilla.org/show_bug.cgi?id=1286861)
  /// * [Microsoft Edge Browser Status](https://developer.microsoft.com/en-us/microsoft-edge/status/samesitecookies/)
  /// * [MS Edge dev blog: "Previewing support for same-site cookies in Microsoft Edge"](https://blogs.windows.com/msedgedev/2018/05/17/samesite-cookies-microsoft-edge-internet-explorer/)
  /// * [Mozilla Bug #1551798: Prototype SameSite=Lax by default](https://bugzilla.mozilla.org/show_bug.cgi?id=1551798)
  /// * [Same-site cookies demonstration by Rowan Merewood](https://peaceful-wing.glitch.me)
  SameSiteCookieAttribute,
  /// Screen Orientation
  ///
  /// Provides the ability to read the screen orientation state, to be informed when this state changes, and to be able to lock the screen orientation to a specific state.
  ///
  /// * [Demo](https://www.audero.it/demo/screen-orientation-api-demo.html)
  /// * [MDN Web Docs - Screen Orientation](https://developer.mozilla.org/en-US/docs/Web/API/Screen.orientation)
  /// * [SitePoint article](https://www.sitepoint.com/introducing-screen-orientation-api/)
  ScreenOrientation,
  /// async attribute for external scripts
  ///
  /// The boolean async attribute on script elements allows the external JavaScript file to run when it's available, without delaying page load first.
  ///
  /// * [MDN Web Docs - Script attributes](https://developer.mozilla.org/en/HTML/Element/script#Attributes)
  /// * [Demo](https://testdrive-archive.azurewebsites.net/Performance/AsyncScripts/Default.html)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/script.js#script-async)
  /// * [async vs defer attributes](https://www.growingwiththeweb.com/2014/02/async-vs-defer-attributes.html)
  ScriptAsync,
  /// defer attribute for external scripts
  ///
  /// The boolean defer attribute on script elements allows the external JavaScript file to run when the DOM is loaded, without delaying page load first.
  ///
  /// * [MDN Web Docs - Script Attributes](https://developer.mozilla.org/en/HTML/Element/script#Attributes)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/script.js#script-defer)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/attributes/defer)
  /// * [async vs defer attributes](https://www.growingwiththeweb.com/2014/02/async-vs-defer-attributes.html)
  ScriptDefer,
  /// scrollIntoView
  ///
  /// The `Element.scrollIntoView()` method scrolls the current element into the visible area of the browser window. Parameters can be provided to set the position inside the visible area as well as whether scrolling should be instant or smooth.
  ///
  /// * [MDN Web Docs - scrollIntoView](https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollIntoView)
  /// * [smooth scroll polyfill : polyfill for smooth behavior option](http://iamdustan.com/smoothscroll/)
  Scrollintoview,
  /// Element.scrollIntoViewIfNeeded()
  ///
  /// If the element is fully within the visible area of the viewport, it does nothing. Otherwise, the element is scrolled into view. A proprietary variant of the standard `Element.scrollIntoView()` method.
  ///
  /// * [Mozilla Bug 403510 - Implement scrollIntoViewIfNeeded](https://bugzilla.mozilla.org/show_bug.cgi?id=403510)
  /// * [W3C CSSOM View bug #17152: Support centering an element when scrolling into view.](https://www.w3.org/Bugs/Public/show_bug.cgi?id=17152)
  Scrollintoviewifneeded,
  /// SDCH Accept-Encoding/Content-Encoding
  ///
  /// Shared Dictionary Compression over HTTP
  ///
  /// * [SDCH Google Group](https://groups.google.com/forum/#!forum/sdch)
  /// * [Bugzilla Bug 641069 - Implement SDCH](https://bugzilla.mozilla.org/show_bug.cgi?id=641069)
  /// * [Wikipedia - SDCH](https://en.wikipedia.org/wiki/SDCH)
  /// * [Shared Dictionary Compression for HTTP at LinkedIn.](https://engineering.linkedin.com/shared-dictionary-compression-http-linkedin)
  Sdch,
  /// Selection API
  ///
  /// API for accessing selected content of a document, including the `window.getSelection()` method, as well as the `selectstart` & `selectionchange` events on `document`.
  ///
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1231923)
  SelectionApi,
  /// Selectlist - Customizable select element
  ///
  /// Proposal for a customizable `<select>` element, currently defined as `<selectlist>`, previously `<selectmenu>`.
  ///
  /// * [Blog post: Two levels of customising <selectlist>](https://hidde.blog/custom-select-with-selectlist/)
  /// * [Open UI's <selectlist> demos](https://microsoftedge.github.io/Demos/selectlist/index.html)
  Selectlist,
  /// Server Timing
  ///
  /// Mechanism for web developers to annotate network requests with server timing information.
  ///
  /// * [Demo](https://server-timing.netlify.com/)
  /// * [Blog post](https://developer.akamai.com/blog/2017/06/07/completing-performance-analysis-server-timing/)
  /// * [MDN article on PerformanceServerTiming](https://developer.mozilla.org/en-US/docs/Web/API/PerformanceServerTiming)
  ServerTiming,
  /// Service Workers
  ///
  /// Method that enables applications to take advantage of persistent background processing, including hooks to enable bootstrapping of web applications while offline.
  ///
  /// * [HTML5Rocks article (introduction)](https://www.html5rocks.com/en/tutorials/service-worker/introduction/)
  /// * [MDN Web Docs - Service Workers](https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorker_API)
  /// * [List of various resources](https://jakearchibald.github.io/isserviceworkerready/resources.html)
  Serviceworkers,
  /// Efficient Script Yielding: setImmediate()
  ///
  /// Yields control flow without the minimum delays enforced by setTimeout
  ///
  /// * [The case for setImmediate()](https://humanwhocodes.com/blog/2013/07/09/the-case-for-setimmediate/)
  /// * [Script yielding with setImmediate](https://humanwhocodes.com/blog/2011/09/19/script-yielding-with-setimmediate/)
  /// * [setImmediate polyfill](https://github.com/YuzuJS/setImmediate)
  /// * [Firefox tracking bug](https://bugzilla.mozilla.org/show_bug.cgi?id=686201)
  /// * [Chrome bug closed as WONTFIX](https://code.google.com/p/chromium/issues/detail?id=146172)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#setimmediate)
  Setimmediate,
  /// Shadow DOM (deprecated V0 spec)
  ///
  /// Original V0 version of the Shadow DOM specification. See [Shadow DOM V1](#feat=shadowdomv1) for support for the latest version.
  ///
  /// * [HTML5Rocks - Shadow DOM 101 article](https://www.html5rocks.com/tutorials/webcomponents/shadowdom/)
  /// * [Safari implementation bug](https://bugs.webkit.org/show_bug.cgi?id=148695)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1205323)
  /// * [Google Developers - Shadow DOM v1: self-contained web components](https://developers.google.com/web/fundamentals/getting-started/primers/shadowdom)
  Shadowdom,
  /// Shadow DOM (V1)
  ///
  /// Method of establishing and maintaining functional boundaries between DOM trees and how these trees interact with each other within a document, thus enabling better functional encapsulation within the DOM & CSS.
  ///
  /// * [Safari implementation bug](https://bugs.webkit.org/show_bug.cgi?id=148695)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1205323)
  /// * [Google Developers - Shadow DOM v1: self-contained web components](https://developers.google.com/web/fundamentals/primers/shadowdom/?hl=en)
  Shadowdomv1,
  /// Shared Array Buffer
  ///
  /// Type of ArrayBuffer that can be shared across Workers.
  ///
  /// * [MDN article](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer)
  /// * [Mozilla Hacks article on safely reviving shared memory](https://hacks.mozilla.org/2020/07/safely-reviving-shared-memory/)
  Sharedarraybuffer,
  /// Shared Web Workers
  ///
  /// Method of allowing multiple scripts to communicate with a single web worker.
  ///
  /// * [Sitepoint article](https://www.sitepoint.com/javascript-shared-web-workers-html5/)
  /// * [Blog post](https://greenido.wordpress.com/2011/11/03/web-workers-part-3-out-of-3-shared-wrokers/)
  Sharedworkers,
  /// Server Name Indication
  ///
  /// An extension to the TLS computer networking protocol by which a client indicates which hostname it is attempting to connect to at the start of the handshaking process.
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/Server_Name_Indication)
  Sni,
  /// SPDY protocol
  ///
  /// Networking protocol for low-latency transport of content over the web. Superseded by HTTP version 2.
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/HTTP/2)
  /// * [SPDY whitepaper](https://dev.chromium.org/spdy/spdy-whitepaper)
  Spdy,
  /// Speech Recognition API
  ///
  /// Method to provide speech input in a web browser.
  ///
  /// * [HTML5Rocks article](https://developer.chrome.com/blog/voice-driven-web-apps-introduction-to-the-web-speech-api/)
  /// * [SitePoint article](https://www.sitepoint.com/introducing-web-speech-api/)
  /// * [Demo](https://www.audero.it/demo/web-speech-api-demo.html)
  /// * [Advanced demo and resource](https://zenorocha.github.io/voice-elements/#recognition-element)
  /// * [Chromium bug to unprefix the Speech Recognition API](https://bugs.chromium.org/p/chromium/issues/detail?id=570968)
  SpeechRecognition,
  /// Speech Synthesis API
  ///
  /// A web API for controlling a text-to-speech output.
  ///
  /// * [SitePoint article](https://www.sitepoint.com/talking-web-pages-and-the-speech-synthesis-api/)
  /// * [Demo](https://www.audero.it/demo/speech-synthesis-api-demo.html)
  /// * [Advanced demo and resource](https://zenorocha.github.io/voice-elements/)
  /// * [MDN article](https://developer.mozilla.org//docs/Web/API/SpeechSynthesis)
  /// * [Google Developers article](https://developers.google.com/web/updates/2014/01/Web-apps-that-talk-Introduction-to-the-Speech-Synthesis-API)
  SpeechSynthesis,
  /// Spellcheck attribute
  ///
  /// Attribute for `input`/`textarea` fields to enable/disable the browser's spellchecker.
  ///
  /// * [MDN Web Docs - Controlling spell checking](https://developer.mozilla.org/en-US/docs/Web/HTML/Controlling_spell_checking_in_HTML_formsControlling_spell_checking_in_HTML_forms)
  SpellcheckAttribute,
  /// Web SQL Database
  ///
  /// Method of storing data client-side, allows SQLite database queries for access and manipulation.
  ///
  /// * [HTML5 Doctor article](https://html5doctor.com/introducing-web-sql-databases/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-sql-db)
  /// * [Chrome platform status: Deprecate and remove WebSQL in non-secure contexts](https://chromestatus.com/feature/5175124599767040)
  SqlStorage,
  /// Srcset and sizes attributes
  ///
  /// The `srcset` and `sizes` attributes on `img` (or `source`) elements allow authors to define various image resources and "hints" that assist a user agent to determine the most appropriate image source to display (e.g. high-resolution displays, small monitors, etc).
  ///
  /// * [Improved support for high-resolution displays with the srcset image attribute](https://www.webkit.org/blog/2910/improved-support-for-high-resolution-displays-with-the-srcset-image-attribute/)
  /// * [Blog post on srcset & sizes](https://ericportis.com/posts/2014/srcset-sizes/)
  /// * [MDN: Responsive images](https://developer.mozilla.org/en-US/docs/Learn/HTML/Multimedia_and_embedding/Responsive_images)
  /// * [MDN: <img> element](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
  Srcset,
  /// getUserMedia/Stream API
  ///
  /// Method of accessing external device data (such as a webcam video stream). Formerly this was envisioned as the <device> element.
  ///
  /// * [Technology preview from Opera](https://dev.opera.com/blog/webcam-orientation-preview/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/dom/Navigator/getUserMedia)
  /// * [Media Capture functionality in Microsoft Edge](https://blogs.windows.com/msedgedev/2015/05/13/announcing-media-capture-functionality-in-microsoft-edge/)
  /// * [getUserMedia in PWA with manifest on iOS 11](https://stackoverflow.com/questions/50800696/getusermedia-in-pwa-with-manifest-on-ios-11)
  /// * [getUserMedia working again in PWA on iOS 13.4](https://bugs.webkit.org/show_bug.cgi?id=185448#c84)
  Stream,
  /// Streams
  ///
  /// Method of creating, composing, and consuming streams of data, that map efficiently to low-level I/O primitives, and allow easy composition with built-in backpressure and queuing.
  ///
  /// * [GitHub repository](https://github.com/whatwg/streams)
  /// * [ReadableStream on Mozilla Developer Network](https://developer.mozilla.org/en/docs/Web/API/ReadableStream)
  /// * [Blog article about streams](https://jakearchibald.com/2016/streams-ftw/)
  Streams,
  /// Strict Transport Security
  ///
  /// Declare that a website is only accessible over a secure connection (HTTPS).
  ///
  /// * [Chromium article](https://www.chromium.org/hsts/)
  /// * [MDN Web Docs - Strict Transport Security](https://developer.mozilla.org/en-US/docs/Security/HTTP_Strict_Transport_Security)
  /// * [OWASP article](https://www.owasp.org/index.php/HTTP_Strict_Transport_Security)
  Stricttransportsecurity,
  /// Scoped attribute
  ///
  /// Deprecated method of allowing scoped CSS styles using a "scoped" attribute. Now [removed from the specification](https://github.com/whatwg/html/issues/552) and replaced by the [@scope CSS rule](/css-cascade-scope).
  ///
  /// * [Polyfill](https://github.com/PM5544/scoped-polyfill)
  /// * [HTML5 Doctor article](https://html5doctor.com/the-scoped-attribute/)
  /// * [HTML5Rocks article](https://developer.chrome.com/blog/a-new-experimental-feature-style-scoped/)
  /// * [Firefox bug #1291515: disable `<style scoped>` in content documents](https://bugzilla.mozilla.org/show_bug.cgi?id=1291515)
  StyleScoped,
  /// Subresource Integrity
  ///
  /// Subresource Integrity enables browsers to verify that file is delivered without unexpected manipulation.
  ///
  /// * [Subresource Integrity (MDN)](https://developer.mozilla.org/en-US/docs/Web/Security/Subresource_Integrity)
  /// * [SRI generation and browser support test](https://www.srihash.org/)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=148363)
  SubresourceIntegrity,
  /// SVG (basic support)
  ///
  /// Method of displaying basic Vector Graphics features using the embed or object elements. Refers to the SVG 1.1 spec.
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics)
  /// * [A List Apart article](https://alistapart.com/article/using-svg-for-flexible-scalable-and-fun-backgrounds-part-i/)
  /// * [SVG showcase site](http://svg-wow.org/)
  /// * [Web-based SVG editor](https://github.com/SVG-Edit/svgedit)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/graphics.js#svg)
  Svg,
  /// SVG in CSS backgrounds
  ///
  /// Method of using SVG images as CSS backgrounds
  ///
  /// * [Tutorial for advanced effects](https://www.sitepoint.com/a-farewell-to-css3-gradients/)
  SvgCss,
  /// SVG filters
  ///
  /// Method of using Photoshop-like effects on SVG objects including blurring and color manipulation.
  ///
  /// * [SVG filter demos](http://svg-wow.org/blog/category/filters/)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/svg/elements/filter)
  /// * [SVG Filter effects](https://jorgeatgu.github.io/svg-filters/)
  SvgFilters,
  /// SVG fonts
  ///
  /// Method of using fonts defined as SVG shapes. Removed from [SVG 2.0](https://www.w3.org/TR/SVG2/changes.html#fonts) and considered as a deprecated feature with support being removed from browsers.
  ///
  /// * [Blog post](http://jeremie.patonnier.net/post/2011/02/07/Why-are-SVG-Fonts-so-different)
  /// * [Blog post on usage for iPad](https://opentype.info/blog/2010/04/13/the-ipad-and-svg-fonts-in-mobile-safari.html?redirect=true)
  SvgFonts,
  /// SVG fragment identifiers
  ///
  /// Method of displaying only a part of an SVG image by defining a view ID or view box dimensions as the file's fragment identifier.
  ///
  /// * [Blog post](http://www.broken-links.com/2012/08/14/better-svg-sprites-with-fragment-identifiers/)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=91791)
  SvgFragment,
  /// SVG effects for HTML
  ///
  /// Method of using SVG transforms, filters, etc on HTML elements using either CSS or the foreignObject element
  ///
  /// * [MDN Web Docs - Other content in SVG](https://developer.mozilla.org/en/SVG/Tutorial/Other_content_in_SVG)
  /// * [MDN Web Docs - Applying SVG effects](https://developer.mozilla.org/en-US/docs/Web/SVG/Applying_SVG_effects_to_HTML_content)
  /// * [Filter Effects draft](https://www.w3.org/TR/filter-effects/)
  SvgHtml,
  /// Inline SVG in HTML5
  ///
  /// Method of using SVG tags directly in HTML documents. Requires HTML5 parser.
  ///
  /// * [Mozilla Hacks blog post](https://hacks.mozilla.org/2010/05/firefox-4-the-html5-parser-inline-svg-speed-and-more/)
  /// * [Test suite](http://samples.msdn.microsoft.com/ietestcenter/html5/svghtml_harness.htm?url=SVG_HTML_Elements_001)
  SvgHtml5,
  /// SVG in HTML img element
  ///
  /// Method of displaying SVG images in HTML using <img>.
  ///
  /// * [Blog with SVGs and images](https://www.codedread.com/blog/)
  SvgImg,
  /// SVG SMIL animation
  ///
  /// Method of using animation elements to animate SVG images
  ///
  /// * [Examples on SVG WOW](http://svg-wow.org/blog/category/animation/)
  /// * [MDN Web Docs - animation with SMIL](https://developer.mozilla.org/en/SVG/SVG_animation_with_SMIL)
  /// * [JS library to support SMIL in SVG](https://leunen.me/fakesmile/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/graphics.js#svg-smil)
  /// * [Polyfill for SMIL animate events on SVG](https://github.com/madsgraphics/SVGEventListener)
  SvgSmil,
  /// Signed HTTP Exchanges (SXG)
  ///
  /// Part of the Web Packaging spec, Signed HTTP Exchanges allow a different origin server to provide a resource, and this will be treated as if it came from the original server. This can be used with AMP CDNs, for example, to allow the original URL to be displayed in the URL bar.
  ///
  /// * [Chrome platform status - Shipped](https://www.chromestatus.com/feature/5745285984681984)
  /// * [Microsoft Edge Platform Status - Supported](https://developer.microsoft.com/en-us/microsoft-edge/status/originsignedhttpexchanges/)
  /// * [Signed HTTP Exchanges on Google's Web Development site](https://developers.google.com/web/updates/2018/11/signed-exchanges)
  /// * [Developer Preview of better AMP URLs in Google Search](https://blog.amp.dev/2018/11/13/developer-preview-of-better-amp-urls-in-google-search/)
  /// * [Signed-Exchange: Solving the AMP URLs Display Problem](https://medium.com/oyotech/implementing-signed-exchange-for-better-amp-urls-38abd64c6766)
  /// * [GitHub home page for Web Packaging](https://github.com/WICG/webpackage)
  /// * [Mozilla's Position about Signed HTTP Exchanges (harmful)](https://mozilla.github.io/standards-positions/#http-origin-signed-responses)
  Sxg,
  /// tabindex global attribute
  ///
  /// Specifies the focusability of the element and in what order (if any) it should become focused (relative to other elements) when "tabbing" through the document.
  ///
  /// * [MDN Web Docs - tabindex attribute](https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex)
  TabindexAttr,
  /// HTML templates
  ///
  /// Method of declaring a portion of reusable markup that is parsed but not rendered until cloned.
  ///
  /// * [web.dev - HTML's New template Tag](https://web.dev/webcomponents-template/)
  /// * [Polyfill script](https://github.com/manubb/template)
  /// * [Template element on MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
  Template,
  /// ES6 Template Literals (Template Strings)
  ///
  /// Template literals are string literals allowing embedded expressions using backtick characters (`). You can use multi-line strings and string interpolation features with them. Formerly known as template strings.
  ///
  /// * [MDN Web Docs - Template literals](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Template_literals)
  /// * [ES6 Template Literals in Depth](https://ponyfoo.com/articles/es6-template-strings-in-depth)
  TemplateLiterals,
  /// Temporal
  ///
  /// A modern API for working with date and time, meant to supersede the original `Date` API.
  ///
  /// * [Fixing JavaScript Date](https://maggiepint.com/2017/04/11/fixing-javascript-date-web-compatibility-and-reality/)
  /// * [Chromium implementation bug](https://bugs.chromium.org/p/v8/issues/detail?id=11544)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1519167)
  /// * [WebKit implementation bug](https://bugs.webkit.org/show_bug.cgi?id=223166)
  /// * [Blog post: Temporal: getting started with JavaScript’s new date time API](https://2ality.com/2021/06/temporal-api.html)
  Temporal,
  /// text-decoration styling
  ///
  /// Method of defining the type, style and color of lines in the text-decoration property. These can be defined as shorthand (e.g. `text-decoration: line-through dashed blue`) or as single properties (e.g. `text-decoration-color: blue`)
  ///
  /// * [MDN Web Docs - text-decoration-style](https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-style)
  /// * [MDN Web Docs - text-decoration-color](https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-color)
  /// * [MDN Web Docs - text-decoration-line](https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-line)
  /// * [MDN Web Docs - text-decoration-skip](https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-skip)
  /// * [Firefox implementation bug](https://bugzilla.mozilla.org/show_bug.cgi?id=812990)
  TextDecoration,
  /// text-emphasis styling
  ///
  /// Method of using small symbols next to each glyph to emphasize a run of text, commonly used in East Asian languages. The `text-emphasis` shorthand, and its `text-emphasis-style` and `text-emphasis-color` longhands, can be used to apply marks to the text. The `text-emphasis-position` property, which inherits separately, allows setting the emphasis marks' position with respect to the text.
  ///
  /// * [A javascript fallback for CSS3 emphasis mark.](https://github.com/zmmbreeze/jquery.emphasis/)
  /// * [MDN Web Docs - text-emphasis](https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis)
  /// * [Chromium bug to unprefix `-webkit-text-emphasis`](https://bugs.chromium.org/p/chromium/issues/detail?id=666433)
  TextEmphasis,
  /// CSS3 Text-overflow
  ///
  /// Append ellipsis when text overflows its containing element
  ///
  /// * [jQuery polyfill for Firefox](https://github.com/rmorse/AutoEllipsis)
  /// * [MDN Web Docs - text-overflow](https://developer.mozilla.org/en-US/docs/Web/CSS/text-overflow)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/css.js#css-text-overflow)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/text-overflow)
  TextOverflow,
  /// CSS text-size-adjust
  ///
  /// On mobile devices, the text-size-adjust CSS property allows Web authors to control if and how the text-inflating algorithm is applied to the textual content of the element it is applied to.
  ///
  /// * [MDN Web Docs - text-size-adjust](https://developer.mozilla.org/en-US/docs/Web/CSS/text-size-adjust)
  /// * [Mozilla Bug #1226116: Unprefix -moz-text-size-adjust](https://bugzilla.mozilla.org/show_bug.cgi?id=1226116)
  TextSizeAdjust,
  /// CSS text-stroke and text-fill
  ///
  /// Method of declaring the outline (stroke) width and color for text.
  ///
  /// * [Information & workarounds](https://css-tricks.com/adding-stroke-to-web-text/)
  /// * [Live editor](https://www.westciv.com/tools/textStroke/)
  /// * [MDN Web Docs - text-stroke](https://developer.mozilla.org/en-US/docs/Web/CSS/-webkit-text-stroke)
  TextStroke,
  /// Node.textContent
  ///
  /// DOM Node property representing the text content of a node and its descendants
  ///
  /// * [MDN Web Docs - Node.textContent](https://developer.mozilla.org/en-US/docs/Web/API/Node/textContent)
  Textcontent,
  /// TextEncoder & TextDecoder
  ///
  /// `TextEncoder` encodes a JavaScript string into bytes using the UTF-8 encoding and returns the resulting `Uint8Array` of those bytes. `TextDecoder` does the reverse.
  ///
  /// * [MDN Web Docs - TextEncoder](https://developer.mozilla.org/en-US/docs/Web/API/TextEncoder)
  /// * [WebKit Bug 160653 - Support TextEncoder & TextDecoder APIs](https://bugs.webkit.org/show_bug.cgi?id=160653)
  Textencoder,
  /// TLS 1.1
  ///
  /// Version 1.1 of the Transport Layer Security (TLS) protocol.
  ///
  /// * [Wikipedia article about TLS 1.1](https://en.wikipedia.org/wiki/Transport_Layer_Security#TLS_1.1)
  /// * [Modernizing Transport Security - Google Security Blog](https://security.googleblog.com/2018/10/modernizing-transport-security.html)
  /// * [Modernizing TLS connections in Microsoft Edge and Internet Explorer 11 - Microsoft Windows Blog](https://blogs.windows.com/msedgedev/2018/10/15/modernizing-tls-edge-ie11/)
  /// * [Removing Old Versions of TLS - Mozilla Security Blog](https://blog.mozilla.org/security/2018/10/15/removing-old-versions-of-tls/)
  /// * [Deprecation of Legacy TLS 1.0 and 1.1 Versions - WebKit Blog](https://webkit.org/blog/8462/deprecation-of-legacy-tls-1-0-and-1-1-versions/)
  Tls11,
  /// TLS 1.2
  ///
  /// Version 1.2 of the Transport Layer Security (TLS) protocol. Allows for data/message confidentiality, and message authentication codes for message integrity and as a by-product message authentication.
  ///
  /// * [Wikipedia article about TLS 1.2](https://en.wikipedia.org/wiki/Transport_Layer_Security#TLS_1.2)
  Tls12,
  /// TLS 1.3
  ///
  /// Version 1.3 (the latest one) of the Transport Layer Security (TLS) protocol. Removes weaker elliptic curves and hash functions.
  ///
  /// * [Wikipedia article about TLS 1.3](https://en.wikipedia.org/wiki/Transport_Layer_Security#TLS_1.3)
  /// * [Chrome support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=630147)
  Tls13,
  /// Touch events
  ///
  /// Method of registering when, where and how the interface is touched, for devices with a touch screen. These DOM events are similar to mousedown, mousemove, etc.
  ///
  /// * [Detailed support tables](https://www.quirksmode.org/mobile/tableTouch.html)
  /// * [Multi-touch demo](https://www.quirksmode.org/m/tests/drag2.html)
  /// * [Information on the spec development](http://schepers.cc/getintouch)
  /// * [Internet Explorer's gesture and touch implementation.](https://docs.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/dev-guides/hh673557(v=vs.85))
  /// * [Touch polyfill for supporting touch events on Internet Explorer](https://github.com/CamHenlin/TouchPolyfill)
  /// * [MDN – Touch events](https://developer.mozilla.org/en-US/docs/Web/API/Touch_events)
  Touch,
  /// CSS3 2D Transforms
  ///
  /// Method of transforming an element including rotating, scaling, etc. Includes support for `transform` as well as `transform-origin` properties.
  ///
  /// * [Live editor](https://www.westciv.com/tools/transforms/)
  /// * [MDN Web Docs - CSS transform](https://developer.mozilla.org/en-US/docs/Web/CSS/transform)
  /// * [Workaround script for IE](http://www.webresourcesdepot.com/cross-browser-css-transforms-csssandpaper/)
  /// * [Converter for IE](https://www.useragentman.com/IETransformsTranslator/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/css.js#css-transform)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/transform/)
  /// * [Microsoft Edge Platform Status (SVG)](https://developer.microsoft.com/en-us/microsoft-edge/status/supportcsstransformsonsvg/)
  Transforms2d,
  /// CSS3 3D Transforms
  ///
  /// Method of transforming an element in the third dimension using the `transform` property. Includes support for the `perspective` property to set the perspective in z-space and the `backface-visibility` property to toggle display of the reverse side of a 3D-transformed element.
  ///
  /// * [Multi-browser demo](http://css3.bradshawenterprises.com/flip/)
  /// * [Mozilla hacks article](https://hacks.mozilla.org/2011/10/css-3d-transformations-in-firefox-nightly/)
  /// * [3D CSS Tester](http://thewebrocks.com/demos/3D-css-tester/)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/css.js#css-transform)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/transform/)
  /// * [Intro to CSS 3D transforms](https://3dtransforms.desandro.com/)
  Transforms3d,
  /// Trusted Types for DOM manipulation
  ///
  /// An API that forces developers to be very explicit about their use of powerful DOM-injection APIs. Can greatly improve security against XSS attacks.
  ///
  /// * [Web.dev article on using trusted types](https://web.dev/trusted-types/)
  /// * [Firefox position: non-harmful](https://mozilla.github.io/standards-positions/#trusted-types)
  TrustedTypes,
  /// TTF/OTF - TrueType and OpenType font support
  ///
  /// Support for the TrueType (.ttf) and OpenType (.otf) outline font formats in @font-face.
  ///
  /// * [What is the status of TTF support in Internet Explorer?](https://stackoverflow.com/questions/17694143/what-is-the-status-of-ttf-support-in-internet-explorer)
  /// * [OTF Specification](https://docs.microsoft.com/en-us/typography/opentype/spec)
  Ttf,
  /// Typed Arrays
  ///
  /// JavaScript typed arrays provide a mechanism for accessing raw binary data much more efficiently. Includes: `Int8Array`, `Uint8Array`, `Uint8ClampedArray`, `Int16Array`, `Uint16Array`, `Int32Array`, `Uint32Array`, `Float32Array` & `Float64Array`
  ///
  ///
  /// * [MDN Web Docs - Typed arrays](https://developer.mozilla.org/en/javascript_typed_arrays)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#ecmascript-typed-arrays)
  Typedarrays,
  /// FIDO U2F API
  ///
  /// JavaScript API to interact with Universal Second Factor (U2F) devices. This allows users to log into sites more securely using two-factor authentication with a USB dongle.
  ///
  /// * [Mozilla bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1065729)
  /// * [Google Security article](https://security.googleblog.com/2014/10/strengthening-2-step-verification-with.html)
  /// * [Chrome platform status: U2F Security Key API removal (Cryptotoken Component Extension)](https://chromestatus.com/feature/5759004926017536)
  /// * [Yubico blog post about the decommission](https://www.yubico.com/blog/google-chrome-u2f-api-decommission-what-the-change-means-for-your-users-and-how-to-prepare/)
  /// * [Chromium Intent to Deprecate and Remove: U2F API (Cryptotoken)](https://groups.google.com/a/chromium.org/g/blink-dev/c/xHC3AtU_65A/m/yg20tsVFBAAJ)
  /// * [Mozilla Firefox bug to remove the U2F API](https://bugzilla.mozilla.org/show_bug.cgi?id=1737205)
  U2f,
  /// unhandledrejection/rejectionhandled events
  ///
  /// The `unhandledrejection` event is fired when a Promise is rejected but there is no rejection handler to deal with the rejection. The `rejectionhandled` event is fired when a Promise is rejected, and after the rejection is handled by the promise's rejection handling code.
  ///
  /// * [MDN article on rejectionhandled](https://developer.mozilla.org/en-US/docs/Web/Events/rejectionhandled)
  /// * [MDN article on unhandledrejection](https://developer.mozilla.org/en-US/docs/Web/Events/unhandledrejection)
  /// * [Chrome sample code](https://googlechrome.github.io/samples/promise-rejection-events/)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#unhandled-rejection-tracking)
  Unhandledrejection,
  /// Upgrade Insecure Requests
  ///
  /// Declare that browsers should transparently upgrade HTTP resources on a website to HTTPS.
  ///
  /// * [MDN Web Docs - Upgrade Insecure Requests](https://developer.mozilla.org/en-US/docs/Web/Security/CSP/CSP_policy_directives#upgrade-insecure-requests)
  /// * [Demo Website](https://googlechrome.github.io/samples/csp-upgrade-insecure-requests/index.html)
  /// * [WebKit feature request bug](https://bugs.webkit.org/show_bug.cgi?id=143653)
  Upgradeinsecurerequests,
  /// URL API
  ///
  /// API to retrieve the various parts that make up a given URL from a given URL string.
  ///
  /// * [MDN Web Docs - URL interface](https://developer.mozilla.org/en-US/docs/Web/API/URL)
  /// * [MDN Web Docs - URL constructor](https://developer.mozilla.org/en-US/docs/Web/API/URL/URL)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#url-and-urlsearchparams)
  Url,
  /// URL Scroll-To-Text Fragment
  ///
  /// URL fragment that defines a piece of text to be scrolled into view and highlighted.
  ///
  /// * [Current Firefox position](https://mozilla.github.io/standards-positions/#scroll-to-text-fragment)
  /// * [Safari's position as of Jan 2020](https://lists.webkit.org/pipermail/webkit-dev/2019-December/030996.html)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1753933)
  /// * [Article about why the `:~:` syntax](https://blog.jim-nielsen.com/2022/scroll-to-text-fragments/)
  UrlScrollToTextFragment,
  /// URLSearchParams
  ///
  /// The URLSearchParams interface defines utility methods to work with the query string of a URL.
  ///
  /// * [Easy URL manipulation with URLSearchParams](https://developers.google.com/web/updates/2016/01/urlsearchparams?hl=en)
  /// * [MDN Web Docs - URLSearchParams](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams)
  /// * [Polyfill for this feature is available in the core-js library](https://github.com/zloirock/core-js#url-and-urlsearchparams)
  /// * [EdgeHTML implementation bug](https://web.archive.org/web/20190624214230/https://developer.microsoft.com/en-us/microsoft-edge/platform/issues/8993198/)
  Urlsearchparams,
  /// ECMAScript 5 Strict Mode
  ///
  /// Method of placing code in a "strict" operating context.
  ///
  /// * [Information page](https://johnresig.com/blog/ecmascript-5-strict-mode-json-and-more/)
  /// * [Article with test suite](https://javascriptweblog.wordpress.com/2011/05/03/javascript-strict-mode/)
  UseStrict,
  /// CSS user-select: none
  ///
  /// Method of preventing text/element selection using CSS.
  ///
  /// * [MDN Web Docs - CSS user-select](https://developer.mozilla.org/en-US/docs/CSS/user-select)
  /// * [CSS Tricks article](https://css-tricks.com/almanac/properties/u/user-select/)
  /// * [MSDN Documentation](https://docs.microsoft.com/en-us/previous-versions/hh781492(v=vs.85))
  /// * [WebKit bug to unprefix `-webkit-user-select`](https://bugs.webkit.org/show_bug.cgi?id=208677)
  UserSelectNone,
  /// User Timing API
  ///
  /// Method to help web developers measure the performance of their applications by giving them access to high precision timestamps.
  ///
  /// * [SitePoint article](https://www.sitepoint.com/discovering-user-timing-api/)
  /// * [HTML5Rocks article](https://www.html5rocks.com/en/tutorials/webperformance/usertiming/)
  /// * [Polyfill](https://gist.github.com/pmeenan/5902672)
  /// * [Demo](https://audero.it/demo/user-timing-api-demo.html)
  /// * [UserTiming.js polyfill](https://github.com/nicjansma/usertiming.js)
  UserTiming,
  /// Variable fonts
  ///
  /// OpenType font settings that allows a single font file to behave like multiple fonts: it can contain all the allowed variations in width, weight, slant, optical size, or any other exposed axes of variation as defined by the font designer. Variations can be applied via the `font-variation-settings` property.
  ///
  /// * [MDN Web docs article](https://developer.mozilla.org/en-US/docs/Web/CSS/font-variation-settings)
  /// * [How to use variable fonts in the real world](https://medium.com/clear-left-thinking/how-to-use-variable-fonts-in-the-real-world-e6d73065a604)
  /// * [v-fonts.com - A simple resource for finding and trying variable fonts](https://v-fonts.com)
  /// * [Axis-Praxis - Tool & info on variable fonts](https://www.axis-praxis.org/about)
  /// * [Getting started with Variable Fonts on the Frontend (2022)](https://evilmartians.com/chronicles/the-joy-of-variable-fonts-getting-started-on-the-frontend)
  VariableFonts,
  /// SVG vector-effect: non-scaling-stroke
  ///
  /// The `non-scaling-stroke` value for the `vector-effect` SVG attribute/CSS property makes strokes appear as the same width regardless of any transformations applied.
  ///
  /// * [MDN Docs article on vector-effect](https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vector-effect)
  /// * [Firefox implementation bug for other values](https://bugzilla.mozilla.org/show_bug.cgi?id=1318208)
  /// * [Chromium implementation bug for other values](https://bugs.chromium.org/p/chromium/issues/detail?id=691398)
  VectorEffect,
  /// Vibration API
  ///
  /// Method to access the vibration mechanism of the hosting device.
  ///
  /// * [MDN Web Docs - Vibration](https://developer.mozilla.org/en-US/docs/Web/Guide/API/Vibration)
  /// * [Vibration API sample code & demo](https://davidwalsh.name/vibration-api)
  /// * [Tuts+ article](https://code.tutsplus.com/tutorials/html5-vibration-api--mobile-22585)
  /// * [Demo](https://audero.it/demo/vibration-api-demo.html)
  /// * [Article and Usage Examples](https://www.illyism.com/journal/vibrate-mobile-phone-web-vibration-api/)
  Vibration,
  /// Video element
  ///
  /// Method of playing videos on webpages (without requiring a plug-in). Includes support for the following media properties: `currentSrc`, `currentTime`, `paused`, `playbackRate`, `buffered`, `duration`, `played`, `seekable`, `ended`, `autoplay`, `loop`, `controls`, `volume` & `muted`
  ///
  /// * [Detailed article on video/audio elements](https://dev.opera.com/articles/everything-you-need-to-know-html5-video-audio/)
  /// * [WebM format information](https://www.webmproject.org)
  /// * [Video for Everybody](http://camendesign.co.uk/code/video_for_everybody)
  /// * [Video on the Web - includes info on Android support](http://diveintohtml5.info/video.html)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/video.js#video)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/html/elements/video)
  Video,
  /// Video Tracks
  ///
  /// Method of specifying and selecting between multiple video tracks. Useful for providing sign language tracks, burnt-in captions or subtitles, alternative camera angles, etc.
  ///
  /// * [MDN Web Docs - HTMLMediaElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement)
  Videotracks,
  /// View Transitions API (single-document)
  ///
  /// Provides a mechanism for easily creating animated transitions between different DOM states, while also updating the DOM contents in a single step. This API is specific to single-document transitions, support for same-origin cross-document transitions is [being planned](https://github.com/WICG/view-transitions/blob/main/cross-doc-explainer.md).
  ///
  /// * [Explainer document](https://github.com/WICG/view-transitions/blob/main/explainer.md)
  /// * [View Transitions API on MDN](https://developer.mozilla.org/en-US/docs/Web/API/View_Transitions_API)
  /// * [Chrome Developers documentation](https://developer.chrome.com/docs/web-platform/view-transitions/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1823896)
  ViewTransitions,
  /// Small, Large, and Dynamic viewport units
  ///
  /// Viewport units similar to `vw` and `vh` that are based on shown or hidden browser UI states to address shortcomings of the original units. Currently defined as the `sv*` units (`svb`, `svh`, `svi`, `svmax`, `svmin`, `svw`), `lv*` units (`lvb`, `lvh`, `lvi`, `lvmax`, `lvmin`, `lvw`), `dv*` units (`dvb`, `dvh`, `dvi`, `dvmax`, `dvmin`, `dvw`) and the logical `vi`/`vb` units.
  ///
  /// * [Blog post explaining the new units](https://www.bram.us/2021/07/08/the-large-small-and-dynamic-viewports/)
  /// * [Chromium support bug](https://bugs.chromium.org/p/chromium/issues/detail?id=1093055)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1610815)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=219287)
  /// * [MDN Web Docs - Relative length units based on viewport](https://developer.mozilla.org/en-US/docs/Web/CSS/length#relative_length_units_based_on_viewport)
  ViewportUnitVariants,
  /// Viewport units: vw, vh, vmin, vmax
  ///
  /// Length units representing a percentage of the current viewport dimensions: width (vw), height (vh), the smaller of the two (vmin), or the larger of the two (vmax).
  ///
  /// * [Blog post](https://css-tricks.com/viewport-sized-typography/)
  /// * [Polyfill](https://github.com/saabi/vminpoly)
  /// * [Buggyfill - Polyfill that fixes buggy support](https://github.com/rodneyrehm/viewport-units-buggyfill)
  /// * [Back-Forward issue blog post](https://blog.rodneyrehm.de/archives/34-iOS7-Mobile-Safari-And-Viewport-Units.html)
  ViewportUnits,
  /// WAI-ARIA Accessibility features
  ///
  /// Method of providing ways for people with disabilities to use dynamic web content and web applications.
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/WAI-ARIA)
  /// * [HTML5/WAI-ARIA information](https://zufelt.ca/blog/are-you-confused-html5-and-wai-aria-yet)
  /// * [a11ysupport.io - Accessibility Support data for various HTML, ARIA, CSS, and SVG features](https://a11ysupport.io/)
  /// * [WAI-ARIA Overview](https://www.w3.org/WAI/standards-guidelines/aria/)
  /// * [Links to various test results](https://developer.paciellogroup.com/blog/2011/10/browser-assistive-technology-tests-redux/)
  /// * [A List Apart - The Accessibility of WAI-ARIA](https://alistapart.com/article/the-accessibility-of-wai-aria/)
  WaiAria,
  /// Screen Wake Lock API
  ///
  /// API to prevent devices from dimming, locking or turning off the screen when the application needs to keep running.
  ///
  /// * [Stay awake with the Screen Wake Lock API](https://web.dev/wakelock/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1589554)
  /// * [WebKit support bug](https://bugs.webkit.org/show_bug.cgi?id=205104)
  /// * [MDN article about the Screen Wake Lock API](https://developer.mozilla.org/en-US/docs/Web/API/Screen_Wake_Lock_API)
  WakeLock,
  /// WebAssembly
  ///
  /// WebAssembly or "wasm" is a new portable, size- and load-time-efficient format suitable for compilation to the web.
  ///
  /// * [Official site](https://webassembly.org/)
  /// * [WebAssembly on MDN](https://developer.mozilla.org/docs/WebAssembly)
  /// * [Roadmap and detailed feature support table](https://webassembly.org/roadmap/)
  Wasm,
  /// WebAssembly BigInt to i64 conversion in JS API
  ///
  /// An extension to the WebAssembly JS API for bidrectionally converting BigInts and 64-bit WebAssembly integer values
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/JS-BigInt-integration)
  WasmBigint,
  /// WebAssembly Bulk Memory Operations
  ///
  /// An extension to WebAssembly adding bulk memory operations and conditional segment initialization
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/bulk-memory-operations/blob/master/proposals/bulk-memory-operations/Overview.md)
  WasmBulkMemory,
  /// WebAssembly Multi-Value
  ///
  /// An extension to WebAssembly allowing instructions, blocks and functions to produce multiple result values
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/multi-value/blob/master/proposals/multi-value/Overview.md)
  WasmMultiValue,
  /// WebAssembly Import/Export of Mutable Globals
  ///
  /// An extension to WebAssembly import and export of mutable global variables
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/mutable-global/blob/master/proposals/mutable-global/Overview.md)
  WasmMutableGlobals,
  /// WebAssembly Non-trapping float-to-int Conversion
  ///
  /// An extension to WebAssembly adding floating-point to integer conversion operators which saturate instead of trapping
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/nontrapping-float-to-int-conversions/blob/main/proposals/nontrapping-float-to-int-conversion/Overview.md)
  WasmNontrappingFptoint,
  /// WebAssembly Reference Types
  ///
  /// An extension to WebAssembly allowing opaque references as first-class types, and multiple tables
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/spec/blob/main/proposals/reference-types/Overview.md)
  WasmReferenceTypes,
  /// WebAssembly Sign Extension Operators
  ///
  /// An extension to WebAssembly adding sign-extension operator instructions
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/sign-extension-ops/blob/master/proposals/sign-extension-ops/Overview.md)
  WasmSignext,
  /// WebAssembly SIMD
  ///
  /// An extension to WebAssembly adding 128-bit SIMD operations
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/simd/blob/main/proposals/simd/SIMD.md)
  WasmSimd,
  /// WebAssembly Threads and Atomics
  ///
  /// An extension to WebAssembly adding shared memory and atomic memory operations
  ///
  /// * [Feature extension overview](https://github.com/WebAssembly/threads/blob/main/proposals/threads/Overview.md)
  WasmThreads,
  /// Wav audio format
  ///
  /// Waveform Audio File Format, aka WAV or WAVE, a typically uncompressed audio format.
  ///
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/WAV)
  Wav,
  /// wbr (word break opportunity) element
  ///
  /// Represents an extra place where a line of text may optionally be broken.
  ///
  /// * [MDN Web Docs - Element wbr](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
  WbrElement,
  /// Web Animations API
  ///
  /// Lets you create animations that are run in the browser, as well as inspect and manipulate animations created through declarative means like CSS.
  ///
  /// * [HTML5 Rocks](https://developer.chrome.com/blog/web-animations-element-animate-is-now-in-chrome-36/)
  /// * [HTML5 Rocks](https://developer.chrome.com/blog/new-web-animations-engine-in-blink-drives-css-animations-transitions/)
  /// * [Current Firefox status](https://birtles.github.io/areweanimatedyet/)
  /// * [Polyfill](https://github.com/web-animations/web-animations-js)
  WebAnimation,
  /// Web Bluetooth
  ///
  /// Allows web sites to communicate over GATT with nearby user-selected Bluetooth devices in a secure and privacy-preserving way.
  ///
  /// * [Intro](https://developers.google.com/web/updates/2015/07/interact-with-ble-devices-on-the-web)
  /// * [Samples](https://googlechrome.github.io/samples/web-bluetooth/)
  /// * [Demos](https://github.com/WebBluetoothCG/demos)
  /// * [Implementation Status](https://github.com/WebBluetoothCG/web-bluetooth/blob/main/implementation-status.md)
  /// * [Mozilla Specification Positions: Harmful](https://mozilla.github.io/standards-positions/#web-bluetooth)
  WebBluetooth,
  /// Web Serial API
  ///
  /// Allows communication with devices via a serial interface.
  ///
  /// * [Explainer](https://github.com/WICG/serial/blob/main/EXPLAINER.md)
  /// * [Read from and write to a serial port](https://web.dev/serial/)
  /// * [Mozilla position: harmful](https://mozilla.github.io/standards-positions/#webserial)
  /// * [WebKit position: opposed](https://webkit.org/tracking-prevention/)
  WebSerial,
  /// Web Share API
  ///
  /// A way to allow websites to invoke the native sharing capabilities of the host platform
  ///
  /// * [Chrome post](https://developers.google.com/web/updates/2016/10/navigator-share)
  /// * [Hospodarets - Web Share API brings the native sharing capabilities to the browser](https://blog.hospodarets.com/web-share-api)
  /// * [Phil Nash - The Web Share API](https://philna.sh/blog/2017/03/14/the-web-share-api/)
  /// * [Mozilla bug #1312422: Consider experimental support for Web Share API](https://bugzilla.mozilla.org/show_bug.cgi?id=1312422)
  /// * [How to Use the Web Share API to Trigger the Native Dialog to Share Content & Pull Quotes](https://love2dev.com/blog/webshare-api/)
  /// * [Web.dev - Share like a native app with the Web Share API](https://web.dev/web-share/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1653481)
  /// * [W3C Demo](https://w3c.github.io/web-share/demos/share.html)
  /// * [Chromium support bug for macOS](https://bugs.chromium.org/p/chromium/issues/detail?id=1144920)
  /// * [Web Share API on MDN](https://developer.mozilla.org/en-US/docs/Web/API/Web_Share_API)
  WebShare,
  /// Web Authentication API
  ///
  /// The Web Authentication API is an extension of the Credential Management API that enables strong authentication with public key cryptography, enabling password-less authentication and / or secure second-factor authentication without SMS texts.
  ///
  /// * [Web Authentication on MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/API/Web_Authentication_API)
  /// * [Web Authentication and Windows Hello](https://docs.microsoft.com/en-us/microsoft-edge/dev-guide/device/web-authentication)
  /// * [Guide to Web Authentication](https://webauthn.guide)
  Webauthn,
  /// WebCodecs API
  ///
  /// API to provide more control over the encoding and decoding of audio, video, and images.
  ///
  /// * [Explainer document](https://github.com/w3c/webcodecs/blob/main/explainer.md)
  /// * [Video processing with WebCodecs](https://web.dev/webcodecs/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=WebCodecs)
  /// * [WebCodecs API on MDN](https://developer.mozilla.org/en-US/docs/Web/API/WebCodecs_API)
  /// * [WebCodecs samples](https://w3c.github.io/webcodecs/samples/)
  Webcodecs,
  /// WebGL - 3D Canvas graphics
  ///
  /// Method of generating dynamic 3D graphics using JavaScript, accelerated through hardware
  ///
  /// * [Instructions on enabling WebGL](https://get.webgl.org/get-a-webgl-implementation/)
  /// * [Tutorial](https://www.khronos.org/webgl/wiki/Tutorial)
  /// * [Firefox blog post](https://hacks.mozilla.org/2009/12/webgl-draft-released-today/)
  /// * [Polyfill for IE](https://github.com/iewebgl/iewebgl)
  Webgl,
  /// WebGL 2.0
  ///
  /// Next version of WebGL. Based on OpenGL ES 3.0.
  ///
  /// * [Firefox blog post](https://blog.mozilla.org/futurereleases/2015/03/03/an-early-look-at-webgl-2/)
  /// * [Getting a WebGL Implementation](https://www.khronos.org/webgl/wiki/Getting_a_WebGL_Implementation)
  Webgl2,
  /// WebGPU
  ///
  /// An API for complex rendering and compute, using hardware acceleration. Use cases include demanding 3D games and acceleration of scientific calculations. Meant to supersede WebGL.
  ///
  /// * [Implementation status](https://github.com/gpuweb/gpuweb/wiki/Implementation-Status)
  /// * [Official Wiki](https://github.com/gpuweb/gpuweb/wiki)
  /// * [WebGPU test scene](https://toji.github.io/webgpu-test/)
  Webgpu,
  /// WebHID API
  ///
  /// Enables raw access to HID (Human Interface Device) commands for all connected HIDs. Previously, an HID could only be accessed if the browser had implemented a custom API for the specific device.
  ///
  /// * [Human interface devices on the web: a few quick examples](https://web.dev/hid-examples/)
  Webhid,
  /// CSS -webkit-user-drag property
  ///
  /// The non-standard `-webkit-user-drag` CSS property can be used to either make an element draggable or explicitly non-draggable (like links and images). See the standardized [draggable attribute/property](/mdn-api_htmlelement_draggable) for the recommended alternative method of accomplishing the same functionality.
  ///
  /// * [Reference](https://css-infos.net/property/-webkit-user-drag)
  WebkitUserDrag,
  /// WebM video format
  ///
  /// Multimedia format designed to provide a royalty-free, high-quality open video compression format for use with HTML5 video. WebM supports the video codec VP8 and VP9.
  ///
  /// * [Official website](https://www.webmproject.org)
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/WebM)
  Webm,
  /// Web NFC
  ///
  /// This API allows a website to communicate with NFC tags through a device's NFC reader.
  ///
  /// * [Safari position: Opposed](https://lists.webkit.org/pipermail/webkit-dev/2020-January/031007.html)
  /// * [Firefox position: Harmful](https://mozilla.github.io/standards-positions/#web-nfc)
  /// * [Web.dev article on using WebNFC](https://web.dev/nfc/)
  Webnfc,
  /// WebP image format
  ///
  /// Image format (based on the VP8 video format) that supports lossy and lossless compression, as well as animation and alpha transparency. WebP generally has better compression than JPEG, PNG and GIF and is designed to supersede them. [AVIF](/avif) and [JPEG XL](/jpegxl) are designed to supersede WebP.
  ///
  /// * [Official website](https://developers.google.com/speed/webp/)
  /// * [Official website FAQ - Which web browsers natively support WebP?](https://developers.google.com/speed/webp/faq#which_web_browsers_natively_support_webp)
  /// * [Bitsofcode - Why and how to use WebP images today](https://bitsofco.de/why-and-how-to-use-webp-images-today/)
  /// * [WebP decoder and encoder](https://github.com/webmproject/libwebp)
  Webp,
  /// Web Sockets
  ///
  /// Bidirectional communication technology for web apps
  ///
  /// * [Details on newer protocol](https://developer.chrome.com/blog/what-s-different-in-the-new-websocket-protocol/)
  /// * [Wikipedia](https://en.wikipedia.org/wiki/WebSocket)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-websockets)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/websocket)
  /// * [MDN Web Docs - WebSockets API](https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API)
  Websockets,
  /// WebTransport
  ///
  /// Protocol framework to send and receive data from servers using [HTTP3](/http3). Similar to [WebSockets](/websockets) but with support for multiple streams, unidirectional streams, out-of-order delivery, and reliable as well as unreliable transport.
  ///
  /// * [web.dev article](https://web.dev/webtransport/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=1709355)
  /// * [Explainer with examples](https://github.com/w3c/webtransport/blob/main/explainer.md)
  /// * [WebKit position on WebTransport](https://github.com/WebKit/standards-positions/issues/18)
  Webtransport,
  /// WebUSB
  ///
  /// Allows communication with devices via USB (Universal Serial Bus).
  ///
  /// * [Google Developers article](https://developers.google.com/web/updates/2016/03/access-usb-devices-on-the-web)
  /// * [Mozilla Specification Positions: Harmful](https://mozilla.github.io/standards-positions/#webusb)
  Webusb,
  /// WebVR API
  ///
  /// API for accessing virtual reality (VR) devices, including sensors and head-mounted displays. Replaced by the [WebXR Device API](/webxr).
  ///
  /// * [Detailed device support information](https://webvr.rocks/)
  /// * [WebVR polyfill](https://github.com/googlevr/webvr-polyfill)
  /// * [WebVR framework](https://aframe.io)
  /// * [WebVR info](https://webvr.info/)
  /// * [MDN Web Docs - WebVR API](https://developer.mozilla.org/en-US/docs/Web/API/WebVR_API)
  /// * [Chrome Platform Status for WebXR Device API](https://www.chromestatus.com/feature/5680169905815552)
  Webvr,
  /// WebVTT - Web Video Text Tracks
  ///
  /// Format for marking up text captions for multimedia resources.
  ///
  /// * [Getting Started With the Track Element](https://www.html5rocks.com/en/tutorials/track/basics/)
  /// * [An Introduction to WebVTT and track](https://dev.opera.com/articles/view/an-introduction-to-webvtt-and-track/)
  /// * [MDN Web Docs - WebVTT](https://developer.mozilla.org/en-US/docs/Web/API/WebVTT_API)
  Webvtt,
  /// Web Workers
  ///
  /// Method of running scripts in the background, isolated from the web page
  ///
  /// * [MDN Web Docs - Using Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers)
  /// * [Web Worker demo](https://nerget.com/rayjs-mt/rayjs.html)
  /// * [Polyfill for IE (single threaded)](https://code.google.com/archive/p/ie-web-worker/)
  /// * [Tutorial](https://code.tutsplus.com/tutorials/getting-started-with-web-workers--net-27667)
  Webworkers,
  /// WebXR Device API
  ///
  /// API for accessing virtual reality (VR) and augmented reality (AR) devices, including sensors and head-mounted displays.
  ///
  /// * [MDN Web Docs - WebXR Device API](https://developer.mozilla.org/docs/Web/API/WebXR_Device_API)
  /// * [Immersive Web - WebXR samples](https://immersive-web.github.io/webxr-samples/)
  /// * [Safari implementation bug](https://bugs.webkit.org/show_bug.cgi?id=208988)
  Webxr,
  /// CSS will-change property
  ///
  /// Method of optimizing animations by informing the browser which elements will change and what properties will change.
  ///
  /// * [Detailed article](https://dev.opera.com/articles/css-will-change-property/)
  /// * [Blog post](https://aerotwist.com/blog/bye-bye-layer-hacks/)
  /// * [MDN Web Docs - will-change](https://developer.mozilla.org/en-US/docs/Web/CSS/will-change)
  WillChange,
  /// WOFF - Web Open Font Format
  ///
  /// Compressed TrueType/OpenType font that contains information about the font's source.
  ///
  /// * [Mozilla hacks blog post](https://hacks.mozilla.org/2009/10/woff/)
  Woff,
  /// WOFF 2.0 - Web Open Font Format
  ///
  /// TrueType/OpenType font that provides better compression than WOFF 1.0.
  ///
  /// * [Basics about WOFF 2.0](https://gist.github.com/sergejmueller/cf6b4f2133bcb3e2f64a)
  /// * [WOFF 2.0 converter](https://everythingfonts.com/ttf-to-woff2)
  Woff2,
  /// CSS3 word-break
  ///
  /// Property to prevent or allow words to be broken over multiple lines between letters.
  ///
  /// * [MDN Web Docs - CSS word-break](https://developer.mozilla.org/en/CSS/word-break)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/word-break)
  WordBreak,
  /// CSS3 Overflow-wrap
  ///
  /// Allows lines to be broken within words if an otherwise unbreakable string is too long to fit. Currently mostly supported using the `word-wrap` property.
  ///
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/css/properties/word-wrap)
  /// * [Bug on Firefox support](https://bugzilla.mozilla.org/show_bug.cgi?id=955857)
  /// * [MDN Web Docs - CSS overflow-wrap](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-wrap)
  Wordwrap,
  /// Cross-document messaging
  ///
  /// Method of sending information from a page on one domain to a page on a different one (using postMessage)
  ///
  /// * [MDN Web Docs - window.postMessage](https://developer.mozilla.org/en/DOM/window.postMessage)
  /// * [Simple demo](https://html5demos.com/postmessage2)
  /// * [has.js test](https://raw.github.com/phiggins42/has.js/master/detect/features.js#native-crosswindowmessaging)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/web-messaging/MessagePort/postMessage)
  XDocMessaging,
  /// X-Frame-Options HTTP header
  ///
  /// An HTTP header which indicates whether the browser should allow the webpage to be displayed in a frame within another webpage. Used as a defense against clickjacking attacks.
  ///
  /// * [X-Frame-Options Compatibility Test](https://erlend.oftedal.no/blog/tools/xframeoptions/)
  /// * [MDN Web Docs - X-Frame-Options](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/X-Frame-Options)
  /// * [OWASP Clickjacking Defense Cheat Sheet](https://www.owasp.org/index.php/Clickjacking_Defense_Cheat_Sheet)
  /// * [Combating ClickJacking With X-Frame-Options - IEInternals](https://blogs.msdn.microsoft.com/ieinternals/2010/03/30/combating-clickjacking-with-x-frame-options/)
  /// * [IE8 Security Part VII: ClickJacking Defenses - IEBlog](https://blogs.msdn.microsoft.com/ie/2009/01/27/ie8-security-part-vii-clickjacking-defenses/)
  XFrameOptions,
  /// XMLHttpRequest advanced features
  ///
  /// Updated functionality to the original XHR specification including things like file uploads, transfer progress information and the ability to send FormData. Previously known as [XMLHttpRequest Level 2](https://www.w3.org/TR/2012/WD-XMLHttpRequest-20120117/), these features now appear simply in the XMLHttpRequest spec.
  ///
  /// * [MDN Web Docs - FormData](https://developer.mozilla.org/en/XMLHttpRequest/FormData)
  /// * [Polyfill for FormData object](https://github.com/jimmywarting/FormData)
  /// * [WebPlatform Docs](https://webplatform.github.io/docs/apis/xhr/XMLHttpRequest)
  Xhr2,
  /// XHTML served as application/xhtml+xml
  ///
  /// A strict form of HTML, and allows embedding of other XML languages
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/XHTML)
  /// * [Information on XHTML5](http://www.xmlplease.com/xhtml/xhtml5polyglot/)
  Xhtml,
  /// XHTML+SMIL animation
  ///
  /// Method of using SMIL animation in web pages
  ///
  /// * [Wikipedia](https://en.wikipedia.org/wiki/XHTML%2BSMIL)
  /// * [JS library to support XHTML+SMIL](https://leunen.me/fakesmile/)
  Xhtmlsmil,
  /// DOM Parsing and Serialization
  ///
  /// Various DOM parsing and serializing functions, specifically `DOMParser`, `XMLSerializer`, `innerHTML`, `outerHTML` and `insertAdjacentHTML`.
  ///
  /// * [MDN Web Docs - XMLSerializer](https://developer.mozilla.org/en-US/docs/XMLSerializer)
  /// * [Comparing Document Position by John Resig](https://johnresig.com/blog/dom-insertadjacenthtml/)
  XmlSerializer,
  /// zstd (Zstandard) content-encoding
  ///
  /// Data compression method providing faster page loading while using less CPU power on the server.
  ///
  /// * [Official Zstandard site](https://facebook.github.io/zstd/)
  /// * [Wikipedia article](https://en.wikipedia.org/wiki/Zstd)
  /// * [Support test](https://www.daniel.priv.no/tools/zstd-browser-test/)
  /// * [Firefox support bug](https://bugzilla.mozilla.org/show_bug.cgi?id=zstd)
  /// * [WebKit position on zstd](https://github.com/WebKit/standards-positions/issues/168)
  Zstd,
  /// Any other browser feature
  Any(String),
}
impl BrowserFeature {
  pub fn key(&self) -> &str {
    match self {
      BrowserFeature::Aac => "aac",
      BrowserFeature::Abortcontroller => "abortcontroller",
      BrowserFeature::Accelerometer => "accelerometer",
      BrowserFeature::Addeventlistener => "addeventlistener",
      BrowserFeature::AmbientLight => "ambient-light",
      BrowserFeature::Apng => "apng",
      BrowserFeature::ArrayFind => "array-find",
      BrowserFeature::ArrayFindIndex => "array-find-index",
      BrowserFeature::ArrayFlat => "array-flat",
      BrowserFeature::ArrayIncludes => "array-includes",
      BrowserFeature::ArrowFunctions => "arrow-functions",
      BrowserFeature::Asmjs => "asmjs",
      BrowserFeature::AsyncClipboard => "async-clipboard",
      BrowserFeature::AsyncFunctions => "async-functions",
      BrowserFeature::AtobBtoa => "atob-btoa",
      BrowserFeature::Audio => "audio",
      BrowserFeature::AudioApi => "audio-api",
      BrowserFeature::Audiotracks => "audiotracks",
      BrowserFeature::Autofocus => "autofocus",
      BrowserFeature::Auxclick => "auxclick",
      BrowserFeature::Av1 => "av1",
      BrowserFeature::Avif => "avif",
      BrowserFeature::BackgroundAttachment => "background-attachment",
      BrowserFeature::BackgroundClipText => "background-clip-text",
      BrowserFeature::BackgroundImgOpts => "background-img-opts",
      BrowserFeature::BackgroundPositionXY => "background-position-x-y",
      BrowserFeature::BackgroundRepeatRoundSpace => "background-repeat-round-space",
      BrowserFeature::BackgroundSync => "background-sync",
      BrowserFeature::BatteryStatus => "battery-status",
      BrowserFeature::Beacon => "beacon",
      BrowserFeature::Beforeafterprint => "beforeafterprint",
      BrowserFeature::Bigint => "bigint",
      BrowserFeature::Blobbuilder => "blobbuilder",
      BrowserFeature::Bloburls => "bloburls",
      BrowserFeature::BorderImage => "border-image",
      BrowserFeature::BorderRadius => "border-radius",
      BrowserFeature::Broadcastchannel => "broadcastchannel",
      BrowserFeature::Brotli => "brotli",
      BrowserFeature::Calc => "calc",
      BrowserFeature::Canvas => "canvas",
      BrowserFeature::CanvasBlending => "canvas-blending",
      BrowserFeature::CanvasText => "canvas-text",
      BrowserFeature::ChUnit => "ch-unit",
      BrowserFeature::Chacha20Poly1305 => "chacha20-poly1305",
      BrowserFeature::ChannelMessaging => "channel-messaging",
      BrowserFeature::ChildnodeRemove => "childnode-remove",
      BrowserFeature::Classlist => "classlist",
      BrowserFeature::ClientHintsDprWidthViewport => "client-hints-dpr-width-viewport",
      BrowserFeature::Clipboard => "clipboard",
      BrowserFeature::Colr => "colr",
      BrowserFeature::ColrV1 => "colr-v1",
      BrowserFeature::Comparedocumentposition => "comparedocumentposition",
      BrowserFeature::ConsoleBasic => "console-basic",
      BrowserFeature::ConsoleTime => "console-time",
      BrowserFeature::Const => "const",
      BrowserFeature::ConstraintValidation => "constraint-validation",
      BrowserFeature::Contenteditable => "contenteditable",
      BrowserFeature::Contentsecuritypolicy => "contentsecuritypolicy",
      BrowserFeature::Contentsecuritypolicy2 => "contentsecuritypolicy2",
      BrowserFeature::CookieStoreApi => "cookie-store-api",
      BrowserFeature::Cors => "cors",
      BrowserFeature::Createimagebitmap => "createimagebitmap",
      BrowserFeature::CredentialManagement => "credential-management",
      BrowserFeature::Cryptography => "cryptography",
      BrowserFeature::CssAll => "css-all",
      BrowserFeature::CssAnchorPositioning => "css-anchor-positioning",
      BrowserFeature::CssAnimation => "css-animation",
      BrowserFeature::CssAnyLink => "css-any-link",
      BrowserFeature::CssAppearance => "css-appearance",
      BrowserFeature::CssAtCounterStyle => "css-at-counter-style",
      BrowserFeature::CssBackdropFilter => "css-backdrop-filter",
      BrowserFeature::CssBackgroundOffsets => "css-background-offsets",
      BrowserFeature::CssBackgroundblendmode => "css-backgroundblendmode",
      BrowserFeature::CssBoxdecorationbreak => "css-boxdecorationbreak",
      BrowserFeature::CssBoxshadow => "css-boxshadow",
      BrowserFeature::CssCanvas => "css-canvas",
      BrowserFeature::CssCaretColor => "css-caret-color",
      BrowserFeature::CssCascadeLayers => "css-cascade-layers",
      BrowserFeature::CssCascadeScope => "css-cascade-scope",
      BrowserFeature::CssCaseInsensitive => "css-case-insensitive",
      BrowserFeature::CssClipPath => "css-clip-path",
      BrowserFeature::CssColorAdjust => "css-color-adjust",
      BrowserFeature::CssColorFunction => "css-color-function",
      BrowserFeature::CssConicGradients => "css-conic-gradients",
      BrowserFeature::CssContainerQueries => "css-container-queries",
      BrowserFeature::CssContainerQueriesStyle => "css-container-queries-style",
      BrowserFeature::CssContainerQueryUnits => "css-container-query-units",
      BrowserFeature::CssContainment => "css-containment",
      BrowserFeature::CssContentVisibility => "css-content-visibility",
      BrowserFeature::CssCounters => "css-counters",
      BrowserFeature::CssCrispEdges => "css-crisp-edges",
      BrowserFeature::CssCrossFade => "css-cross-fade",
      BrowserFeature::CssDefaultPseudo => "css-default-pseudo",
      BrowserFeature::CssDescendantGtgt => "css-descendant-gtgt",
      BrowserFeature::CssDeviceadaptation => "css-deviceadaptation",
      BrowserFeature::CssDirPseudo => "css-dir-pseudo",
      BrowserFeature::CssDisplayContents => "css-display-contents",
      BrowserFeature::CssElementFunction => "css-element-function",
      BrowserFeature::CssEnvFunction => "css-env-function",
      BrowserFeature::CssExclusions => "css-exclusions",
      BrowserFeature::CssFeaturequeries => "css-featurequeries",
      BrowserFeature::CssFilterFunction => "css-filter-function",
      BrowserFeature::CssFilters => "css-filters",
      BrowserFeature::CssFirstLetter => "css-first-letter",
      BrowserFeature::CssFirstLine => "css-first-line",
      BrowserFeature::CssFixed => "css-fixed",
      BrowserFeature::CssFocusVisible => "css-focus-visible",
      BrowserFeature::CssFocusWithin => "css-focus-within",
      BrowserFeature::CssFontPalette => "css-font-palette",
      BrowserFeature::CssFontRenderingControls => "css-font-rendering-controls",
      BrowserFeature::CssFontStretch => "css-font-stretch",
      BrowserFeature::CssGencontent => "css-gencontent",
      BrowserFeature::CssGradients => "css-gradients",
      BrowserFeature::CssGrid => "css-grid",
      BrowserFeature::CssHangingPunctuation => "css-hanging-punctuation",
      BrowserFeature::CssHas => "css-has",
      BrowserFeature::CssHyphens => "css-hyphens",
      BrowserFeature::CssImageOrientation => "css-image-orientation",
      BrowserFeature::CssImageSet => "css-image-set",
      BrowserFeature::CssInOutOfRange => "css-in-out-of-range",
      BrowserFeature::CssIndeterminatePseudo => "css-indeterminate-pseudo",
      BrowserFeature::CssInitialLetter => "css-initial-letter",
      BrowserFeature::CssInitialValue => "css-initial-value",
      BrowserFeature::CssLchLab => "css-lch-lab",
      BrowserFeature::CssLetterSpacing => "css-letter-spacing",
      BrowserFeature::CssLineClamp => "css-line-clamp",
      BrowserFeature::CssLogicalProps => "css-logical-props",
      BrowserFeature::CssMarkerPseudo => "css-marker-pseudo",
      BrowserFeature::CssMasks => "css-masks",
      BrowserFeature::CssMatchesPseudo => "css-matches-pseudo",
      BrowserFeature::CssMathFunctions => "css-math-functions",
      BrowserFeature::CssMediaInteraction => "css-media-interaction",
      BrowserFeature::CssMediaRangeSyntax => "css-media-range-syntax",
      BrowserFeature::CssMediaResolution => "css-media-resolution",
      BrowserFeature::CssMediaqueries => "css-mediaqueries",
      BrowserFeature::CssMixblendmode => "css-mixblendmode",
      BrowserFeature::CssMotionPaths => "css-motion-paths",
      BrowserFeature::CssNamespaces => "css-namespaces",
      BrowserFeature::CssNesting => "css-nesting",
      BrowserFeature::CssNotSelList => "css-not-sel-list",
      BrowserFeature::CssNthChildOf => "css-nth-child-of",
      BrowserFeature::CssOpacity => "css-opacity",
      BrowserFeature::CssOptionalPseudo => "css-optional-pseudo",
      BrowserFeature::CssOverflow => "css-overflow",
      BrowserFeature::CssOverflowAnchor => "css-overflow-anchor",
      BrowserFeature::CssOverflowOverlay => "css-overflow-overlay",
      BrowserFeature::CssOverscrollBehavior => "css-overscroll-behavior",
      BrowserFeature::CssPageBreak => "css-page-break",
      BrowserFeature::CssPagedMedia => "css-paged-media",
      BrowserFeature::CssPaintApi => "css-paint-api",
      BrowserFeature::CssPlaceholder => "css-placeholder",
      BrowserFeature::CssPlaceholderShown => "css-placeholder-shown",
      BrowserFeature::CssReadOnlyWrite => "css-read-only-write",
      BrowserFeature::CssRebeccapurple => "css-rebeccapurple",
      BrowserFeature::CssReflections => "css-reflections",
      BrowserFeature::CssRegions => "css-regions",
      BrowserFeature::CssRelativeColors => "css-relative-colors",
      BrowserFeature::CssRepeatingGradients => "css-repeating-gradients",
      BrowserFeature::CssResize => "css-resize",
      BrowserFeature::CssRevertValue => "css-revert-value",
      BrowserFeature::CssRrggbbaa => "css-rrggbbaa",
      BrowserFeature::CssScrollBehavior => "css-scroll-behavior",
      BrowserFeature::CssScrollbar => "css-scrollbar",
      BrowserFeature::CssSel2 => "css-sel2",
      BrowserFeature::CssSel3 => "css-sel3",
      BrowserFeature::CssSelection => "css-selection",
      BrowserFeature::CssShapes => "css-shapes",
      BrowserFeature::CssSnappoints => "css-snappoints",
      BrowserFeature::CssSticky => "css-sticky",
      BrowserFeature::CssSubgrid => "css-subgrid",
      BrowserFeature::CssSupportsApi => "css-supports-api",
      BrowserFeature::CssTable => "css-table",
      BrowserFeature::CssTextAlignLast => "css-text-align-last",
      BrowserFeature::CssTextBoxTrim => "css-text-box-trim",
      BrowserFeature::CssTextIndent => "css-text-indent",
      BrowserFeature::CssTextJustify => "css-text-justify",
      BrowserFeature::CssTextOrientation => "css-text-orientation",
      BrowserFeature::CssTextWrapBalance => "css-text-wrap-balance",
      BrowserFeature::CssTextshadow => "css-textshadow",
      BrowserFeature::CssTouchAction => "css-touch-action",
      BrowserFeature::CssTransitions => "css-transitions",
      BrowserFeature::CssUnsetValue => "css-unset-value",
      BrowserFeature::CssVariables => "css-variables",
      BrowserFeature::CssWhenElse => "css-when-else",
      BrowserFeature::CssWidowsOrphans => "css-widows-orphans",
      BrowserFeature::CssWritingMode => "css-writing-mode",
      BrowserFeature::CssZoom => "css-zoom",
      BrowserFeature::Css3Attr => "css3-attr",
      BrowserFeature::Css3Boxsizing => "css3-boxsizing",
      BrowserFeature::Css3Colors => "css3-colors",
      BrowserFeature::Css3Cursors => "css3-cursors",
      BrowserFeature::Css3CursorsGrab => "css3-cursors-grab",
      BrowserFeature::Css3CursorsNewer => "css3-cursors-newer",
      BrowserFeature::Css3Tabsize => "css3-tabsize",
      BrowserFeature::Currentcolor => "currentcolor",
      BrowserFeature::CustomElements => "custom-elements",
      BrowserFeature::CustomElementsv1 => "custom-elementsv1",
      BrowserFeature::Customevent => "customevent",
      BrowserFeature::Datalist => "datalist",
      BrowserFeature::Dataset => "dataset",
      BrowserFeature::Datauri => "datauri",
      BrowserFeature::DateTolocaledatestring => "date-tolocaledatestring",
      BrowserFeature::DeclarativeShadowDom => "declarative-shadow-dom",
      BrowserFeature::Decorators => "decorators",
      BrowserFeature::Details => "details",
      BrowserFeature::Deviceorientation => "deviceorientation",
      BrowserFeature::Devicepixelratio => "devicepixelratio",
      BrowserFeature::Dialog => "dialog",
      BrowserFeature::Dispatchevent => "dispatchevent",
      BrowserFeature::Dnssec => "dnssec",
      BrowserFeature::DoNotTrack => "do-not-track",
      BrowserFeature::DocumentCurrentscript => "document-currentscript",
      BrowserFeature::DocumentEvaluateXpath => "document-evaluate-xpath",
      BrowserFeature::DocumentExeccommand => "document-execcommand",
      BrowserFeature::DocumentPolicy => "document-policy",
      BrowserFeature::DocumentScrollingelement => "document-scrollingelement",
      BrowserFeature::Documenthead => "documenthead",
      BrowserFeature::DomManipConvenience => "dom-manip-convenience",
      BrowserFeature::DomRange => "dom-range",
      BrowserFeature::Domcontentloaded => "domcontentloaded",
      BrowserFeature::Dommatrix => "dommatrix",
      BrowserFeature::Download => "download",
      BrowserFeature::Dragndrop => "dragndrop",
      BrowserFeature::ElementClosest => "element-closest",
      BrowserFeature::ElementFromPoint => "element-from-point",
      BrowserFeature::ElementScrollMethods => "element-scroll-methods",
      BrowserFeature::Eme => "eme",
      BrowserFeature::Eot => "eot",
      BrowserFeature::Es5 => "es5",
      BrowserFeature::Es6 => "es6",
      BrowserFeature::Es6Class => "es6-class",
      BrowserFeature::Es6Generators => "es6-generators",
      BrowserFeature::Es6Module => "es6-module",
      BrowserFeature::Es6ModuleDynamicImport => "es6-module-dynamic-import",
      BrowserFeature::Es6Number => "es6-number",
      BrowserFeature::Es6StringIncludes => "es6-string-includes",
      BrowserFeature::Eventsource => "eventsource",
      BrowserFeature::ExtendedSystemFonts => "extended-system-fonts",
      BrowserFeature::FeaturePolicy => "feature-policy",
      BrowserFeature::Fetch => "fetch",
      BrowserFeature::FieldsetDisabled => "fieldset-disabled",
      BrowserFeature::Fileapi => "fileapi",
      BrowserFeature::Filereader => "filereader",
      BrowserFeature::Filereadersync => "filereadersync",
      BrowserFeature::Filesystem => "filesystem",
      BrowserFeature::Flac => "flac",
      BrowserFeature::Flexbox => "flexbox",
      BrowserFeature::FlexboxGap => "flexbox-gap",
      BrowserFeature::FlowRoot => "flow-root",
      BrowserFeature::FocusinFocusoutEvents => "focusin-focusout-events",
      BrowserFeature::FontFamilySystemUi => "font-family-system-ui",
      BrowserFeature::FontFeature => "font-feature",
      BrowserFeature::FontKerning => "font-kerning",
      BrowserFeature::FontLoading => "font-loading",
      BrowserFeature::FontSizeAdjust => "font-size-adjust",
      BrowserFeature::FontSmooth => "font-smooth",
      BrowserFeature::FontUnicodeRange => "font-unicode-range",
      BrowserFeature::FontVariantAlternates => "font-variant-alternates",
      BrowserFeature::FontVariantNumeric => "font-variant-numeric",
      BrowserFeature::Fontface => "fontface",
      BrowserFeature::FormAttribute => "form-attribute",
      BrowserFeature::FormSubmitAttributes => "form-submit-attributes",
      BrowserFeature::FormValidation => "form-validation",
      BrowserFeature::Fullscreen => "fullscreen",
      BrowserFeature::Gamepad => "gamepad",
      BrowserFeature::Geolocation => "geolocation",
      BrowserFeature::Getboundingclientrect => "getboundingclientrect",
      BrowserFeature::Getcomputedstyle => "getcomputedstyle",
      BrowserFeature::Getelementsbyclassname => "getelementsbyclassname",
      BrowserFeature::Getrandomvalues => "getrandomvalues",
      BrowserFeature::Gyroscope => "gyroscope",
      BrowserFeature::Hardwareconcurrency => "hardwareconcurrency",
      BrowserFeature::Hashchange => "hashchange",
      BrowserFeature::Heif => "heif",
      BrowserFeature::Hevc => "hevc",
      BrowserFeature::Hidden => "hidden",
      BrowserFeature::HighResolutionTime => "high-resolution-time",
      BrowserFeature::History => "history",
      BrowserFeature::HtmlMediaCapture => "html-media-capture",
      BrowserFeature::Html5semantic => "html5semantic",
      BrowserFeature::HttpLiveStreaming => "http-live-streaming",
      BrowserFeature::Http2 => "http2",
      BrowserFeature::Http3 => "http3",
      BrowserFeature::IframeSandbox => "iframe-sandbox",
      BrowserFeature::IframeSeamless => "iframe-seamless",
      BrowserFeature::IframeSrcdoc => "iframe-srcdoc",
      BrowserFeature::Imagecapture => "imagecapture",
      BrowserFeature::Ime => "ime",
      BrowserFeature::ImgNaturalwidthNaturalheight => "img-naturalwidth-naturalheight",
      BrowserFeature::ImportMaps => "import-maps",
      BrowserFeature::Imports => "imports",
      BrowserFeature::IndeterminateCheckbox => "indeterminate-checkbox",
      BrowserFeature::Indexeddb => "indexeddb",
      BrowserFeature::Indexeddb2 => "indexeddb2",
      BrowserFeature::InlineBlock => "inline-block",
      BrowserFeature::Innertext => "innertext",
      BrowserFeature::InputAutocompleteOnoff => "input-autocomplete-onoff",
      BrowserFeature::InputColor => "input-color",
      BrowserFeature::InputDatetime => "input-datetime",
      BrowserFeature::InputEmailTelUrl => "input-email-tel-url",
      BrowserFeature::InputEvent => "input-event",
      BrowserFeature::InputFileAccept => "input-file-accept",
      BrowserFeature::InputFileDirectory => "input-file-directory",
      BrowserFeature::InputFileMultiple => "input-file-multiple",
      BrowserFeature::InputInputmode => "input-inputmode",
      BrowserFeature::InputMinlength => "input-minlength",
      BrowserFeature::InputNumber => "input-number",
      BrowserFeature::InputPattern => "input-pattern",
      BrowserFeature::InputPlaceholder => "input-placeholder",
      BrowserFeature::InputRange => "input-range",
      BrowserFeature::InputSearch => "input-search",
      BrowserFeature::InputSelection => "input-selection",
      BrowserFeature::InsertAdjacent => "insert-adjacent",
      BrowserFeature::Insertadjacenthtml => "insertadjacenthtml",
      BrowserFeature::Internationalization => "internationalization",
      BrowserFeature::Intersectionobserver => "intersectionobserver",
      BrowserFeature::IntersectionobserverV2 => "intersectionobserver-v2",
      BrowserFeature::IntlPluralrules => "intl-pluralrules",
      BrowserFeature::IntrinsicWidth => "intrinsic-width",
      BrowserFeature::Jpeg2000 => "jpeg2000",
      BrowserFeature::Jpegxl => "jpegxl",
      BrowserFeature::Jpegxr => "jpegxr",
      BrowserFeature::JsRegexpLookbehind => "js-regexp-lookbehind",
      BrowserFeature::Json => "json",
      BrowserFeature::JustifyContentSpaceEvenly => "justify-content-space-evenly",
      BrowserFeature::KerningPairsLigatures => "kerning-pairs-ligatures",
      BrowserFeature::KeyboardeventCharcode => "keyboardevent-charcode",
      BrowserFeature::KeyboardeventCode => "keyboardevent-code",
      BrowserFeature::KeyboardeventGetmodifierstate => "keyboardevent-getmodifierstate",
      BrowserFeature::KeyboardeventKey => "keyboardevent-key",
      BrowserFeature::KeyboardeventLocation => "keyboardevent-location",
      BrowserFeature::KeyboardeventWhich => "keyboardevent-which",
      BrowserFeature::Lazyload => "lazyload",
      BrowserFeature::Let => "let",
      BrowserFeature::LinkIconPng => "link-icon-png",
      BrowserFeature::LinkIconSvg => "link-icon-svg",
      BrowserFeature::LinkRelDnsPrefetch => "link-rel-dns-prefetch",
      BrowserFeature::LinkRelModulepreload => "link-rel-modulepreload",
      BrowserFeature::LinkRelPreconnect => "link-rel-preconnect",
      BrowserFeature::LinkRelPrefetch => "link-rel-prefetch",
      BrowserFeature::LinkRelPreload => "link-rel-preload",
      BrowserFeature::LinkRelPrerender => "link-rel-prerender",
      BrowserFeature::LoadingLazyAttr => "loading-lazy-attr",
      BrowserFeature::Localecompare => "localecompare",
      BrowserFeature::Magnetometer => "magnetometer",
      BrowserFeature::Matchesselector => "matchesselector",
      BrowserFeature::Matchmedia => "matchmedia",
      BrowserFeature::Mathml => "mathml",
      BrowserFeature::Maxlength => "maxlength",
      BrowserFeature::MediaFragments => "media-fragments",
      BrowserFeature::MediacaptureFromelement => "mediacapture-fromelement",
      BrowserFeature::Mediarecorder => "mediarecorder",
      BrowserFeature::Mediasource => "mediasource",
      BrowserFeature::Menu => "menu",
      BrowserFeature::MetaThemeColor => "meta-theme-color",
      BrowserFeature::Meter => "meter",
      BrowserFeature::Midi => "midi",
      BrowserFeature::Minmaxwh => "minmaxwh",
      BrowserFeature::Mp3 => "mp3",
      BrowserFeature::MpegDash => "mpeg-dash",
      BrowserFeature::Mpeg4 => "mpeg4",
      BrowserFeature::Multibackgrounds => "multibackgrounds",
      BrowserFeature::Multicolumn => "multicolumn",
      BrowserFeature::MutationEvents => "mutation-events",
      BrowserFeature::Mutationobserver => "mutationobserver",
      BrowserFeature::NamevalueStorage => "namevalue-storage",
      BrowserFeature::NativeFilesystemApi => "native-filesystem-api",
      BrowserFeature::NavTiming => "nav-timing",
      BrowserFeature::Netinfo => "netinfo",
      BrowserFeature::Notifications => "notifications",
      BrowserFeature::ObjectEntries => "object-entries",
      BrowserFeature::ObjectFit => "object-fit",
      BrowserFeature::ObjectObserve => "object-observe",
      BrowserFeature::ObjectValues => "object-values",
      BrowserFeature::Objectrtc => "objectrtc",
      BrowserFeature::OfflineApps => "offline-apps",
      BrowserFeature::Offscreencanvas => "offscreencanvas",
      BrowserFeature::OggVorbis => "ogg-vorbis",
      BrowserFeature::Ogv => "ogv",
      BrowserFeature::OlReversed => "ol-reversed",
      BrowserFeature::OnceEventListener => "once-event-listener",
      BrowserFeature::OnlineStatus => "online-status",
      BrowserFeature::Opus => "opus",
      BrowserFeature::OrientationSensor => "orientation-sensor",
      BrowserFeature::Outline => "outline",
      BrowserFeature::PadStartEnd => "pad-start-end",
      BrowserFeature::PageTransitionEvents => "page-transition-events",
      BrowserFeature::Pagevisibility => "pagevisibility",
      BrowserFeature::PassiveEventListener => "passive-event-listener",
      BrowserFeature::Passkeys => "passkeys",
      BrowserFeature::Path2d => "path2d",
      BrowserFeature::PaymentRequest => "payment-request",
      BrowserFeature::PdfViewer => "pdf-viewer",
      BrowserFeature::PermissionsApi => "permissions-api",
      BrowserFeature::PermissionsPolicy => "permissions-policy",
      BrowserFeature::Picture => "picture",
      BrowserFeature::PictureInPicture => "picture-in-picture",
      BrowserFeature::Ping => "ping",
      BrowserFeature::PngAlpha => "png-alpha",
      BrowserFeature::Pointer => "pointer",
      BrowserFeature::PointerEvents => "pointer-events",
      BrowserFeature::Pointerlock => "pointerlock",
      BrowserFeature::Portals => "portals",
      BrowserFeature::PrefersColorScheme => "prefers-color-scheme",
      BrowserFeature::PrefersReducedMotion => "prefers-reduced-motion",
      BrowserFeature::Progress => "progress",
      BrowserFeature::PromiseFinally => "promise-finally",
      BrowserFeature::Promises => "promises",
      BrowserFeature::Proximity => "proximity",
      BrowserFeature::Proxy => "proxy",
      BrowserFeature::Publickeypinning => "publickeypinning",
      BrowserFeature::PushApi => "push-api",
      BrowserFeature::Queryselector => "queryselector",
      BrowserFeature::ReadonlyAttr => "readonly-attr",
      BrowserFeature::ReferrerPolicy => "referrer-policy",
      BrowserFeature::Registerprotocolhandler => "registerprotocolhandler",
      BrowserFeature::RelNoopener => "rel-noopener",
      BrowserFeature::RelNoreferrer => "rel-noreferrer",
      BrowserFeature::Rellist => "rellist",
      BrowserFeature::Rem => "rem",
      BrowserFeature::Requestanimationframe => "requestanimationframe",
      BrowserFeature::Requestidlecallback => "requestidlecallback",
      BrowserFeature::Resizeobserver => "resizeobserver",
      BrowserFeature::ResourceTiming => "resource-timing",
      BrowserFeature::RestParameters => "rest-parameters",
      BrowserFeature::Rtcpeerconnection => "rtcpeerconnection",
      BrowserFeature::Ruby => "ruby",
      BrowserFeature::RunIn => "run-in",
      BrowserFeature::SameSiteCookieAttribute => "same-site-cookie-attribute",
      BrowserFeature::ScreenOrientation => "screen-orientation",
      BrowserFeature::ScriptAsync => "script-async",
      BrowserFeature::ScriptDefer => "script-defer",
      BrowserFeature::Scrollintoview => "scrollintoview",
      BrowserFeature::Scrollintoviewifneeded => "scrollintoviewifneeded",
      BrowserFeature::Sdch => "sdch",
      BrowserFeature::SelectionApi => "selection-api",
      BrowserFeature::Selectlist => "selectlist",
      BrowserFeature::ServerTiming => "server-timing",
      BrowserFeature::Serviceworkers => "serviceworkers",
      BrowserFeature::Setimmediate => "setimmediate",
      BrowserFeature::Shadowdom => "shadowdom",
      BrowserFeature::Shadowdomv1 => "shadowdomv1",
      BrowserFeature::Sharedarraybuffer => "sharedarraybuffer",
      BrowserFeature::Sharedworkers => "sharedworkers",
      BrowserFeature::Sni => "sni",
      BrowserFeature::Spdy => "spdy",
      BrowserFeature::SpeechRecognition => "speech-recognition",
      BrowserFeature::SpeechSynthesis => "speech-synthesis",
      BrowserFeature::SpellcheckAttribute => "spellcheck-attribute",
      BrowserFeature::SqlStorage => "sql-storage",
      BrowserFeature::Srcset => "srcset",
      BrowserFeature::Stream => "stream",
      BrowserFeature::Streams => "streams",
      BrowserFeature::Stricttransportsecurity => "stricttransportsecurity",
      BrowserFeature::StyleScoped => "style-scoped",
      BrowserFeature::SubresourceIntegrity => "subresource-integrity",
      BrowserFeature::Svg => "svg",
      BrowserFeature::SvgCss => "svg-css",
      BrowserFeature::SvgFilters => "svg-filters",
      BrowserFeature::SvgFonts => "svg-fonts",
      BrowserFeature::SvgFragment => "svg-fragment",
      BrowserFeature::SvgHtml => "svg-html",
      BrowserFeature::SvgHtml5 => "svg-html5",
      BrowserFeature::SvgImg => "svg-img",
      BrowserFeature::SvgSmil => "svg-smil",
      BrowserFeature::Sxg => "sxg",
      BrowserFeature::TabindexAttr => "tabindex-attr",
      BrowserFeature::Template => "template",
      BrowserFeature::TemplateLiterals => "template-literals",
      BrowserFeature::Temporal => "temporal",
      BrowserFeature::TextDecoration => "text-decoration",
      BrowserFeature::TextEmphasis => "text-emphasis",
      BrowserFeature::TextOverflow => "text-overflow",
      BrowserFeature::TextSizeAdjust => "text-size-adjust",
      BrowserFeature::TextStroke => "text-stroke",
      BrowserFeature::Textcontent => "textcontent",
      BrowserFeature::Textencoder => "textencoder",
      BrowserFeature::Tls11 => "tls1-1",
      BrowserFeature::Tls12 => "tls1-2",
      BrowserFeature::Tls13 => "tls1-3",
      BrowserFeature::Touch => "touch",
      BrowserFeature::Transforms2d => "transforms2d",
      BrowserFeature::Transforms3d => "transforms3d",
      BrowserFeature::TrustedTypes => "trusted-types",
      BrowserFeature::Ttf => "ttf",
      BrowserFeature::Typedarrays => "typedarrays",
      BrowserFeature::U2f => "u2f",
      BrowserFeature::Unhandledrejection => "unhandledrejection",
      BrowserFeature::Upgradeinsecurerequests => "upgradeinsecurerequests",
      BrowserFeature::Url => "url",
      BrowserFeature::UrlScrollToTextFragment => "url-scroll-to-text-fragment",
      BrowserFeature::Urlsearchparams => "urlsearchparams",
      BrowserFeature::UseStrict => "use-strict",
      BrowserFeature::UserSelectNone => "user-select-none",
      BrowserFeature::UserTiming => "user-timing",
      BrowserFeature::VariableFonts => "variable-fonts",
      BrowserFeature::VectorEffect => "vector-effect",
      BrowserFeature::Vibration => "vibration",
      BrowserFeature::Video => "video",
      BrowserFeature::Videotracks => "videotracks",
      BrowserFeature::ViewTransitions => "view-transitions",
      BrowserFeature::ViewportUnitVariants => "viewport-unit-variants",
      BrowserFeature::ViewportUnits => "viewport-units",
      BrowserFeature::WaiAria => "wai-aria",
      BrowserFeature::WakeLock => "wake-lock",
      BrowserFeature::Wasm => "wasm",
      BrowserFeature::WasmBigint => "wasm-bigint",
      BrowserFeature::WasmBulkMemory => "wasm-bulk-memory",
      BrowserFeature::WasmMultiValue => "wasm-multi-value",
      BrowserFeature::WasmMutableGlobals => "wasm-mutable-globals",
      BrowserFeature::WasmNontrappingFptoint => "wasm-nontrapping-fptoint",
      BrowserFeature::WasmReferenceTypes => "wasm-reference-types",
      BrowserFeature::WasmSignext => "wasm-signext",
      BrowserFeature::WasmSimd => "wasm-simd",
      BrowserFeature::WasmThreads => "wasm-threads",
      BrowserFeature::Wav => "wav",
      BrowserFeature::WbrElement => "wbr-element",
      BrowserFeature::WebAnimation => "web-animation",
      BrowserFeature::WebBluetooth => "web-bluetooth",
      BrowserFeature::WebSerial => "web-serial",
      BrowserFeature::WebShare => "web-share",
      BrowserFeature::Webauthn => "webauthn",
      BrowserFeature::Webcodecs => "webcodecs",
      BrowserFeature::Webgl => "webgl",
      BrowserFeature::Webgl2 => "webgl2",
      BrowserFeature::Webgpu => "webgpu",
      BrowserFeature::Webhid => "webhid",
      BrowserFeature::WebkitUserDrag => "webkit-user-drag",
      BrowserFeature::Webm => "webm",
      BrowserFeature::Webnfc => "webnfc",
      BrowserFeature::Webp => "webp",
      BrowserFeature::Websockets => "websockets",
      BrowserFeature::Webtransport => "webtransport",
      BrowserFeature::Webusb => "webusb",
      BrowserFeature::Webvr => "webvr",
      BrowserFeature::Webvtt => "webvtt",
      BrowserFeature::Webworkers => "webworkers",
      BrowserFeature::Webxr => "webxr",
      BrowserFeature::WillChange => "will-change",
      BrowserFeature::Woff => "woff",
      BrowserFeature::Woff2 => "woff2",
      BrowserFeature::WordBreak => "word-break",
      BrowserFeature::Wordwrap => "wordwrap",
      BrowserFeature::XDocMessaging => "x-doc-messaging",
      BrowserFeature::XFrameOptions => "x-frame-options",
      BrowserFeature::Xhr2 => "xhr2",
      BrowserFeature::Xhtml => "xhtml",
      BrowserFeature::Xhtmlsmil => "xhtmlsmil",
      BrowserFeature::XmlSerializer => "xml-serializer",
      BrowserFeature::Zstd => "zstd",
      BrowserFeature::Any(key) => key,
    }
  }
  pub fn from_key(key: &str) -> Self {
    match key {
      "aac" => BrowserFeature::Aac,
      "abortcontroller" => BrowserFeature::Abortcontroller,
      "accelerometer" => BrowserFeature::Accelerometer,
      "addeventlistener" => BrowserFeature::Addeventlistener,
      "ambient-light" => BrowserFeature::AmbientLight,
      "apng" => BrowserFeature::Apng,
      "array-find" => BrowserFeature::ArrayFind,
      "array-find-index" => BrowserFeature::ArrayFindIndex,
      "array-flat" => BrowserFeature::ArrayFlat,
      "array-includes" => BrowserFeature::ArrayIncludes,
      "arrow-functions" => BrowserFeature::ArrowFunctions,
      "asmjs" => BrowserFeature::Asmjs,
      "async-clipboard" => BrowserFeature::AsyncClipboard,
      "async-functions" => BrowserFeature::AsyncFunctions,
      "atob-btoa" => BrowserFeature::AtobBtoa,
      "audio" => BrowserFeature::Audio,
      "audio-api" => BrowserFeature::AudioApi,
      "audiotracks" => BrowserFeature::Audiotracks,
      "autofocus" => BrowserFeature::Autofocus,
      "auxclick" => BrowserFeature::Auxclick,
      "av1" => BrowserFeature::Av1,
      "avif" => BrowserFeature::Avif,
      "background-attachment" => BrowserFeature::BackgroundAttachment,
      "background-clip-text" => BrowserFeature::BackgroundClipText,
      "background-img-opts" => BrowserFeature::BackgroundImgOpts,
      "background-position-x-y" => BrowserFeature::BackgroundPositionXY,
      "background-repeat-round-space" => BrowserFeature::BackgroundRepeatRoundSpace,
      "background-sync" => BrowserFeature::BackgroundSync,
      "battery-status" => BrowserFeature::BatteryStatus,
      "beacon" => BrowserFeature::Beacon,
      "beforeafterprint" => BrowserFeature::Beforeafterprint,
      "bigint" => BrowserFeature::Bigint,
      "blobbuilder" => BrowserFeature::Blobbuilder,
      "bloburls" => BrowserFeature::Bloburls,
      "border-image" => BrowserFeature::BorderImage,
      "border-radius" => BrowserFeature::BorderRadius,
      "broadcastchannel" => BrowserFeature::Broadcastchannel,
      "brotli" => BrowserFeature::Brotli,
      "calc" => BrowserFeature::Calc,
      "canvas" => BrowserFeature::Canvas,
      "canvas-blending" => BrowserFeature::CanvasBlending,
      "canvas-text" => BrowserFeature::CanvasText,
      "ch-unit" => BrowserFeature::ChUnit,
      "chacha20-poly1305" => BrowserFeature::Chacha20Poly1305,
      "channel-messaging" => BrowserFeature::ChannelMessaging,
      "childnode-remove" => BrowserFeature::ChildnodeRemove,
      "classlist" => BrowserFeature::Classlist,
      "client-hints-dpr-width-viewport" => BrowserFeature::ClientHintsDprWidthViewport,
      "clipboard" => BrowserFeature::Clipboard,
      "colr" => BrowserFeature::Colr,
      "colr-v1" => BrowserFeature::ColrV1,
      "comparedocumentposition" => BrowserFeature::Comparedocumentposition,
      "console-basic" => BrowserFeature::ConsoleBasic,
      "console-time" => BrowserFeature::ConsoleTime,
      "const" => BrowserFeature::Const,
      "constraint-validation" => BrowserFeature::ConstraintValidation,
      "contenteditable" => BrowserFeature::Contenteditable,
      "contentsecuritypolicy" => BrowserFeature::Contentsecuritypolicy,
      "contentsecuritypolicy2" => BrowserFeature::Contentsecuritypolicy2,
      "cookie-store-api" => BrowserFeature::CookieStoreApi,
      "cors" => BrowserFeature::Cors,
      "createimagebitmap" => BrowserFeature::Createimagebitmap,
      "credential-management" => BrowserFeature::CredentialManagement,
      "cryptography" => BrowserFeature::Cryptography,
      "css-all" => BrowserFeature::CssAll,
      "css-anchor-positioning" => BrowserFeature::CssAnchorPositioning,
      "css-animation" => BrowserFeature::CssAnimation,
      "css-any-link" => BrowserFeature::CssAnyLink,
      "css-appearance" => BrowserFeature::CssAppearance,
      "css-at-counter-style" => BrowserFeature::CssAtCounterStyle,
      "css-backdrop-filter" => BrowserFeature::CssBackdropFilter,
      "css-background-offsets" => BrowserFeature::CssBackgroundOffsets,
      "css-backgroundblendmode" => BrowserFeature::CssBackgroundblendmode,
      "css-boxdecorationbreak" => BrowserFeature::CssBoxdecorationbreak,
      "css-boxshadow" => BrowserFeature::CssBoxshadow,
      "css-canvas" => BrowserFeature::CssCanvas,
      "css-caret-color" => BrowserFeature::CssCaretColor,
      "css-cascade-layers" => BrowserFeature::CssCascadeLayers,
      "css-cascade-scope" => BrowserFeature::CssCascadeScope,
      "css-case-insensitive" => BrowserFeature::CssCaseInsensitive,
      "css-clip-path" => BrowserFeature::CssClipPath,
      "css-color-adjust" => BrowserFeature::CssColorAdjust,
      "css-color-function" => BrowserFeature::CssColorFunction,
      "css-conic-gradients" => BrowserFeature::CssConicGradients,
      "css-container-queries" => BrowserFeature::CssContainerQueries,
      "css-container-queries-style" => BrowserFeature::CssContainerQueriesStyle,
      "css-container-query-units" => BrowserFeature::CssContainerQueryUnits,
      "css-containment" => BrowserFeature::CssContainment,
      "css-content-visibility" => BrowserFeature::CssContentVisibility,
      "css-counters" => BrowserFeature::CssCounters,
      "css-crisp-edges" => BrowserFeature::CssCrispEdges,
      "css-cross-fade" => BrowserFeature::CssCrossFade,
      "css-default-pseudo" => BrowserFeature::CssDefaultPseudo,
      "css-descendant-gtgt" => BrowserFeature::CssDescendantGtgt,
      "css-deviceadaptation" => BrowserFeature::CssDeviceadaptation,
      "css-dir-pseudo" => BrowserFeature::CssDirPseudo,
      "css-display-contents" => BrowserFeature::CssDisplayContents,
      "css-element-function" => BrowserFeature::CssElementFunction,
      "css-env-function" => BrowserFeature::CssEnvFunction,
      "css-exclusions" => BrowserFeature::CssExclusions,
      "css-featurequeries" => BrowserFeature::CssFeaturequeries,
      "css-filter-function" => BrowserFeature::CssFilterFunction,
      "css-filters" => BrowserFeature::CssFilters,
      "css-first-letter" => BrowserFeature::CssFirstLetter,
      "css-first-line" => BrowserFeature::CssFirstLine,
      "css-fixed" => BrowserFeature::CssFixed,
      "css-focus-visible" => BrowserFeature::CssFocusVisible,
      "css-focus-within" => BrowserFeature::CssFocusWithin,
      "css-font-palette" => BrowserFeature::CssFontPalette,
      "css-font-rendering-controls" => BrowserFeature::CssFontRenderingControls,
      "css-font-stretch" => BrowserFeature::CssFontStretch,
      "css-gencontent" => BrowserFeature::CssGencontent,
      "css-gradients" => BrowserFeature::CssGradients,
      "css-grid" => BrowserFeature::CssGrid,
      "css-hanging-punctuation" => BrowserFeature::CssHangingPunctuation,
      "css-has" => BrowserFeature::CssHas,
      "css-hyphens" => BrowserFeature::CssHyphens,
      "css-image-orientation" => BrowserFeature::CssImageOrientation,
      "css-image-set" => BrowserFeature::CssImageSet,
      "css-in-out-of-range" => BrowserFeature::CssInOutOfRange,
      "css-indeterminate-pseudo" => BrowserFeature::CssIndeterminatePseudo,
      "css-initial-letter" => BrowserFeature::CssInitialLetter,
      "css-initial-value" => BrowserFeature::CssInitialValue,
      "css-lch-lab" => BrowserFeature::CssLchLab,
      "css-letter-spacing" => BrowserFeature::CssLetterSpacing,
      "css-line-clamp" => BrowserFeature::CssLineClamp,
      "css-logical-props" => BrowserFeature::CssLogicalProps,
      "css-marker-pseudo" => BrowserFeature::CssMarkerPseudo,
      "css-masks" => BrowserFeature::CssMasks,
      "css-matches-pseudo" => BrowserFeature::CssMatchesPseudo,
      "css-math-functions" => BrowserFeature::CssMathFunctions,
      "css-media-interaction" => BrowserFeature::CssMediaInteraction,
      "css-media-range-syntax" => BrowserFeature::CssMediaRangeSyntax,
      "css-media-resolution" => BrowserFeature::CssMediaResolution,
      "css-mediaqueries" => BrowserFeature::CssMediaqueries,
      "css-mixblendmode" => BrowserFeature::CssMixblendmode,
      "css-motion-paths" => BrowserFeature::CssMotionPaths,
      "css-namespaces" => BrowserFeature::CssNamespaces,
      "css-nesting" => BrowserFeature::CssNesting,
      "css-not-sel-list" => BrowserFeature::CssNotSelList,
      "css-nth-child-of" => BrowserFeature::CssNthChildOf,
      "css-opacity" => BrowserFeature::CssOpacity,
      "css-optional-pseudo" => BrowserFeature::CssOptionalPseudo,
      "css-overflow" => BrowserFeature::CssOverflow,
      "css-overflow-anchor" => BrowserFeature::CssOverflowAnchor,
      "css-overflow-overlay" => BrowserFeature::CssOverflowOverlay,
      "css-overscroll-behavior" => BrowserFeature::CssOverscrollBehavior,
      "css-page-break" => BrowserFeature::CssPageBreak,
      "css-paged-media" => BrowserFeature::CssPagedMedia,
      "css-paint-api" => BrowserFeature::CssPaintApi,
      "css-placeholder" => BrowserFeature::CssPlaceholder,
      "css-placeholder-shown" => BrowserFeature::CssPlaceholderShown,
      "css-read-only-write" => BrowserFeature::CssReadOnlyWrite,
      "css-rebeccapurple" => BrowserFeature::CssRebeccapurple,
      "css-reflections" => BrowserFeature::CssReflections,
      "css-regions" => BrowserFeature::CssRegions,
      "css-relative-colors" => BrowserFeature::CssRelativeColors,
      "css-repeating-gradients" => BrowserFeature::CssRepeatingGradients,
      "css-resize" => BrowserFeature::CssResize,
      "css-revert-value" => BrowserFeature::CssRevertValue,
      "css-rrggbbaa" => BrowserFeature::CssRrggbbaa,
      "css-scroll-behavior" => BrowserFeature::CssScrollBehavior,
      "css-scrollbar" => BrowserFeature::CssScrollbar,
      "css-sel2" => BrowserFeature::CssSel2,
      "css-sel3" => BrowserFeature::CssSel3,
      "css-selection" => BrowserFeature::CssSelection,
      "css-shapes" => BrowserFeature::CssShapes,
      "css-snappoints" => BrowserFeature::CssSnappoints,
      "css-sticky" => BrowserFeature::CssSticky,
      "css-subgrid" => BrowserFeature::CssSubgrid,
      "css-supports-api" => BrowserFeature::CssSupportsApi,
      "css-table" => BrowserFeature::CssTable,
      "css-text-align-last" => BrowserFeature::CssTextAlignLast,
      "css-text-box-trim" => BrowserFeature::CssTextBoxTrim,
      "css-text-indent" => BrowserFeature::CssTextIndent,
      "css-text-justify" => BrowserFeature::CssTextJustify,
      "css-text-orientation" => BrowserFeature::CssTextOrientation,
      "css-text-wrap-balance" => BrowserFeature::CssTextWrapBalance,
      "css-textshadow" => BrowserFeature::CssTextshadow,
      "css-touch-action" => BrowserFeature::CssTouchAction,
      "css-transitions" => BrowserFeature::CssTransitions,
      "css-unset-value" => BrowserFeature::CssUnsetValue,
      "css-variables" => BrowserFeature::CssVariables,
      "css-when-else" => BrowserFeature::CssWhenElse,
      "css-widows-orphans" => BrowserFeature::CssWidowsOrphans,
      "css-writing-mode" => BrowserFeature::CssWritingMode,
      "css-zoom" => BrowserFeature::CssZoom,
      "css3-attr" => BrowserFeature::Css3Attr,
      "css3-boxsizing" => BrowserFeature::Css3Boxsizing,
      "css3-colors" => BrowserFeature::Css3Colors,
      "css3-cursors" => BrowserFeature::Css3Cursors,
      "css3-cursors-grab" => BrowserFeature::Css3CursorsGrab,
      "css3-cursors-newer" => BrowserFeature::Css3CursorsNewer,
      "css3-tabsize" => BrowserFeature::Css3Tabsize,
      "currentcolor" => BrowserFeature::Currentcolor,
      "custom-elements" => BrowserFeature::CustomElements,
      "custom-elementsv1" => BrowserFeature::CustomElementsv1,
      "customevent" => BrowserFeature::Customevent,
      "datalist" => BrowserFeature::Datalist,
      "dataset" => BrowserFeature::Dataset,
      "datauri" => BrowserFeature::Datauri,
      "date-tolocaledatestring" => BrowserFeature::DateTolocaledatestring,
      "declarative-shadow-dom" => BrowserFeature::DeclarativeShadowDom,
      "decorators" => BrowserFeature::Decorators,
      "details" => BrowserFeature::Details,
      "deviceorientation" => BrowserFeature::Deviceorientation,
      "devicepixelratio" => BrowserFeature::Devicepixelratio,
      "dialog" => BrowserFeature::Dialog,
      "dispatchevent" => BrowserFeature::Dispatchevent,
      "dnssec" => BrowserFeature::Dnssec,
      "do-not-track" => BrowserFeature::DoNotTrack,
      "document-currentscript" => BrowserFeature::DocumentCurrentscript,
      "document-evaluate-xpath" => BrowserFeature::DocumentEvaluateXpath,
      "document-execcommand" => BrowserFeature::DocumentExeccommand,
      "document-policy" => BrowserFeature::DocumentPolicy,
      "document-scrollingelement" => BrowserFeature::DocumentScrollingelement,
      "documenthead" => BrowserFeature::Documenthead,
      "dom-manip-convenience" => BrowserFeature::DomManipConvenience,
      "dom-range" => BrowserFeature::DomRange,
      "domcontentloaded" => BrowserFeature::Domcontentloaded,
      "dommatrix" => BrowserFeature::Dommatrix,
      "download" => BrowserFeature::Download,
      "dragndrop" => BrowserFeature::Dragndrop,
      "element-closest" => BrowserFeature::ElementClosest,
      "element-from-point" => BrowserFeature::ElementFromPoint,
      "element-scroll-methods" => BrowserFeature::ElementScrollMethods,
      "eme" => BrowserFeature::Eme,
      "eot" => BrowserFeature::Eot,
      "es5" => BrowserFeature::Es5,
      "es6" => BrowserFeature::Es6,
      "es6-class" => BrowserFeature::Es6Class,
      "es6-generators" => BrowserFeature::Es6Generators,
      "es6-module" => BrowserFeature::Es6Module,
      "es6-module-dynamic-import" => BrowserFeature::Es6ModuleDynamicImport,
      "es6-number" => BrowserFeature::Es6Number,
      "es6-string-includes" => BrowserFeature::Es6StringIncludes,
      "eventsource" => BrowserFeature::Eventsource,
      "extended-system-fonts" => BrowserFeature::ExtendedSystemFonts,
      "feature-policy" => BrowserFeature::FeaturePolicy,
      "fetch" => BrowserFeature::Fetch,
      "fieldset-disabled" => BrowserFeature::FieldsetDisabled,
      "fileapi" => BrowserFeature::Fileapi,
      "filereader" => BrowserFeature::Filereader,
      "filereadersync" => BrowserFeature::Filereadersync,
      "filesystem" => BrowserFeature::Filesystem,
      "flac" => BrowserFeature::Flac,
      "flexbox" => BrowserFeature::Flexbox,
      "flexbox-gap" => BrowserFeature::FlexboxGap,
      "flow-root" => BrowserFeature::FlowRoot,
      "focusin-focusout-events" => BrowserFeature::FocusinFocusoutEvents,
      "font-family-system-ui" => BrowserFeature::FontFamilySystemUi,
      "font-feature" => BrowserFeature::FontFeature,
      "font-kerning" => BrowserFeature::FontKerning,
      "font-loading" => BrowserFeature::FontLoading,
      "font-size-adjust" => BrowserFeature::FontSizeAdjust,
      "font-smooth" => BrowserFeature::FontSmooth,
      "font-unicode-range" => BrowserFeature::FontUnicodeRange,
      "font-variant-alternates" => BrowserFeature::FontVariantAlternates,
      "font-variant-numeric" => BrowserFeature::FontVariantNumeric,
      "fontface" => BrowserFeature::Fontface,
      "form-attribute" => BrowserFeature::FormAttribute,
      "form-submit-attributes" => BrowserFeature::FormSubmitAttributes,
      "form-validation" => BrowserFeature::FormValidation,
      "fullscreen" => BrowserFeature::Fullscreen,
      "gamepad" => BrowserFeature::Gamepad,
      "geolocation" => BrowserFeature::Geolocation,
      "getboundingclientrect" => BrowserFeature::Getboundingclientrect,
      "getcomputedstyle" => BrowserFeature::Getcomputedstyle,
      "getelementsbyclassname" => BrowserFeature::Getelementsbyclassname,
      "getrandomvalues" => BrowserFeature::Getrandomvalues,
      "gyroscope" => BrowserFeature::Gyroscope,
      "hardwareconcurrency" => BrowserFeature::Hardwareconcurrency,
      "hashchange" => BrowserFeature::Hashchange,
      "heif" => BrowserFeature::Heif,
      "hevc" => BrowserFeature::Hevc,
      "hidden" => BrowserFeature::Hidden,
      "high-resolution-time" => BrowserFeature::HighResolutionTime,
      "history" => BrowserFeature::History,
      "html-media-capture" => BrowserFeature::HtmlMediaCapture,
      "html5semantic" => BrowserFeature::Html5semantic,
      "http-live-streaming" => BrowserFeature::HttpLiveStreaming,
      "http2" => BrowserFeature::Http2,
      "http3" => BrowserFeature::Http3,
      "iframe-sandbox" => BrowserFeature::IframeSandbox,
      "iframe-seamless" => BrowserFeature::IframeSeamless,
      "iframe-srcdoc" => BrowserFeature::IframeSrcdoc,
      "imagecapture" => BrowserFeature::Imagecapture,
      "ime" => BrowserFeature::Ime,
      "img-naturalwidth-naturalheight" => BrowserFeature::ImgNaturalwidthNaturalheight,
      "import-maps" => BrowserFeature::ImportMaps,
      "imports" => BrowserFeature::Imports,
      "indeterminate-checkbox" => BrowserFeature::IndeterminateCheckbox,
      "indexeddb" => BrowserFeature::Indexeddb,
      "indexeddb2" => BrowserFeature::Indexeddb2,
      "inline-block" => BrowserFeature::InlineBlock,
      "innertext" => BrowserFeature::Innertext,
      "input-autocomplete-onoff" => BrowserFeature::InputAutocompleteOnoff,
      "input-color" => BrowserFeature::InputColor,
      "input-datetime" => BrowserFeature::InputDatetime,
      "input-email-tel-url" => BrowserFeature::InputEmailTelUrl,
      "input-event" => BrowserFeature::InputEvent,
      "input-file-accept" => BrowserFeature::InputFileAccept,
      "input-file-directory" => BrowserFeature::InputFileDirectory,
      "input-file-multiple" => BrowserFeature::InputFileMultiple,
      "input-inputmode" => BrowserFeature::InputInputmode,
      "input-minlength" => BrowserFeature::InputMinlength,
      "input-number" => BrowserFeature::InputNumber,
      "input-pattern" => BrowserFeature::InputPattern,
      "input-placeholder" => BrowserFeature::InputPlaceholder,
      "input-range" => BrowserFeature::InputRange,
      "input-search" => BrowserFeature::InputSearch,
      "input-selection" => BrowserFeature::InputSelection,
      "insert-adjacent" => BrowserFeature::InsertAdjacent,
      "insertadjacenthtml" => BrowserFeature::Insertadjacenthtml,
      "internationalization" => BrowserFeature::Internationalization,
      "intersectionobserver" => BrowserFeature::Intersectionobserver,
      "intersectionobserver-v2" => BrowserFeature::IntersectionobserverV2,
      "intl-pluralrules" => BrowserFeature::IntlPluralrules,
      "intrinsic-width" => BrowserFeature::IntrinsicWidth,
      "jpeg2000" => BrowserFeature::Jpeg2000,
      "jpegxl" => BrowserFeature::Jpegxl,
      "jpegxr" => BrowserFeature::Jpegxr,
      "js-regexp-lookbehind" => BrowserFeature::JsRegexpLookbehind,
      "json" => BrowserFeature::Json,
      "justify-content-space-evenly" => BrowserFeature::JustifyContentSpaceEvenly,
      "kerning-pairs-ligatures" => BrowserFeature::KerningPairsLigatures,
      "keyboardevent-charcode" => BrowserFeature::KeyboardeventCharcode,
      "keyboardevent-code" => BrowserFeature::KeyboardeventCode,
      "keyboardevent-getmodifierstate" => BrowserFeature::KeyboardeventGetmodifierstate,
      "keyboardevent-key" => BrowserFeature::KeyboardeventKey,
      "keyboardevent-location" => BrowserFeature::KeyboardeventLocation,
      "keyboardevent-which" => BrowserFeature::KeyboardeventWhich,
      "lazyload" => BrowserFeature::Lazyload,
      "let" => BrowserFeature::Let,
      "link-icon-png" => BrowserFeature::LinkIconPng,
      "link-icon-svg" => BrowserFeature::LinkIconSvg,
      "link-rel-dns-prefetch" => BrowserFeature::LinkRelDnsPrefetch,
      "link-rel-modulepreload" => BrowserFeature::LinkRelModulepreload,
      "link-rel-preconnect" => BrowserFeature::LinkRelPreconnect,
      "link-rel-prefetch" => BrowserFeature::LinkRelPrefetch,
      "link-rel-preload" => BrowserFeature::LinkRelPreload,
      "link-rel-prerender" => BrowserFeature::LinkRelPrerender,
      "loading-lazy-attr" => BrowserFeature::LoadingLazyAttr,
      "localecompare" => BrowserFeature::Localecompare,
      "magnetometer" => BrowserFeature::Magnetometer,
      "matchesselector" => BrowserFeature::Matchesselector,
      "matchmedia" => BrowserFeature::Matchmedia,
      "mathml" => BrowserFeature::Mathml,
      "maxlength" => BrowserFeature::Maxlength,
      "media-fragments" => BrowserFeature::MediaFragments,
      "mediacapture-fromelement" => BrowserFeature::MediacaptureFromelement,
      "mediarecorder" => BrowserFeature::Mediarecorder,
      "mediasource" => BrowserFeature::Mediasource,
      "menu" => BrowserFeature::Menu,
      "meta-theme-color" => BrowserFeature::MetaThemeColor,
      "meter" => BrowserFeature::Meter,
      "midi" => BrowserFeature::Midi,
      "minmaxwh" => BrowserFeature::Minmaxwh,
      "mp3" => BrowserFeature::Mp3,
      "mpeg-dash" => BrowserFeature::MpegDash,
      "mpeg4" => BrowserFeature::Mpeg4,
      "multibackgrounds" => BrowserFeature::Multibackgrounds,
      "multicolumn" => BrowserFeature::Multicolumn,
      "mutation-events" => BrowserFeature::MutationEvents,
      "mutationobserver" => BrowserFeature::Mutationobserver,
      "namevalue-storage" => BrowserFeature::NamevalueStorage,
      "native-filesystem-api" => BrowserFeature::NativeFilesystemApi,
      "nav-timing" => BrowserFeature::NavTiming,
      "netinfo" => BrowserFeature::Netinfo,
      "notifications" => BrowserFeature::Notifications,
      "object-entries" => BrowserFeature::ObjectEntries,
      "object-fit" => BrowserFeature::ObjectFit,
      "object-observe" => BrowserFeature::ObjectObserve,
      "object-values" => BrowserFeature::ObjectValues,
      "objectrtc" => BrowserFeature::Objectrtc,
      "offline-apps" => BrowserFeature::OfflineApps,
      "offscreencanvas" => BrowserFeature::Offscreencanvas,
      "ogg-vorbis" => BrowserFeature::OggVorbis,
      "ogv" => BrowserFeature::Ogv,
      "ol-reversed" => BrowserFeature::OlReversed,
      "once-event-listener" => BrowserFeature::OnceEventListener,
      "online-status" => BrowserFeature::OnlineStatus,
      "opus" => BrowserFeature::Opus,
      "orientation-sensor" => BrowserFeature::OrientationSensor,
      "outline" => BrowserFeature::Outline,
      "pad-start-end" => BrowserFeature::PadStartEnd,
      "page-transition-events" => BrowserFeature::PageTransitionEvents,
      "pagevisibility" => BrowserFeature::Pagevisibility,
      "passive-event-listener" => BrowserFeature::PassiveEventListener,
      "passkeys" => BrowserFeature::Passkeys,
      "path2d" => BrowserFeature::Path2d,
      "payment-request" => BrowserFeature::PaymentRequest,
      "pdf-viewer" => BrowserFeature::PdfViewer,
      "permissions-api" => BrowserFeature::PermissionsApi,
      "permissions-policy" => BrowserFeature::PermissionsPolicy,
      "picture" => BrowserFeature::Picture,
      "picture-in-picture" => BrowserFeature::PictureInPicture,
      "ping" => BrowserFeature::Ping,
      "png-alpha" => BrowserFeature::PngAlpha,
      "pointer" => BrowserFeature::Pointer,
      "pointer-events" => BrowserFeature::PointerEvents,
      "pointerlock" => BrowserFeature::Pointerlock,
      "portals" => BrowserFeature::Portals,
      "prefers-color-scheme" => BrowserFeature::PrefersColorScheme,
      "prefers-reduced-motion" => BrowserFeature::PrefersReducedMotion,
      "progress" => BrowserFeature::Progress,
      "promise-finally" => BrowserFeature::PromiseFinally,
      "promises" => BrowserFeature::Promises,
      "proximity" => BrowserFeature::Proximity,
      "proxy" => BrowserFeature::Proxy,
      "publickeypinning" => BrowserFeature::Publickeypinning,
      "push-api" => BrowserFeature::PushApi,
      "queryselector" => BrowserFeature::Queryselector,
      "readonly-attr" => BrowserFeature::ReadonlyAttr,
      "referrer-policy" => BrowserFeature::ReferrerPolicy,
      "registerprotocolhandler" => BrowserFeature::Registerprotocolhandler,
      "rel-noopener" => BrowserFeature::RelNoopener,
      "rel-noreferrer" => BrowserFeature::RelNoreferrer,
      "rellist" => BrowserFeature::Rellist,
      "rem" => BrowserFeature::Rem,
      "requestanimationframe" => BrowserFeature::Requestanimationframe,
      "requestidlecallback" => BrowserFeature::Requestidlecallback,
      "resizeobserver" => BrowserFeature::Resizeobserver,
      "resource-timing" => BrowserFeature::ResourceTiming,
      "rest-parameters" => BrowserFeature::RestParameters,
      "rtcpeerconnection" => BrowserFeature::Rtcpeerconnection,
      "ruby" => BrowserFeature::Ruby,
      "run-in" => BrowserFeature::RunIn,
      "same-site-cookie-attribute" => BrowserFeature::SameSiteCookieAttribute,
      "screen-orientation" => BrowserFeature::ScreenOrientation,
      "script-async" => BrowserFeature::ScriptAsync,
      "script-defer" => BrowserFeature::ScriptDefer,
      "scrollintoview" => BrowserFeature::Scrollintoview,
      "scrollintoviewifneeded" => BrowserFeature::Scrollintoviewifneeded,
      "sdch" => BrowserFeature::Sdch,
      "selection-api" => BrowserFeature::SelectionApi,
      "selectlist" => BrowserFeature::Selectlist,
      "server-timing" => BrowserFeature::ServerTiming,
      "serviceworkers" => BrowserFeature::Serviceworkers,
      "setimmediate" => BrowserFeature::Setimmediate,
      "shadowdom" => BrowserFeature::Shadowdom,
      "shadowdomv1" => BrowserFeature::Shadowdomv1,
      "sharedarraybuffer" => BrowserFeature::Sharedarraybuffer,
      "sharedworkers" => BrowserFeature::Sharedworkers,
      "sni" => BrowserFeature::Sni,
      "spdy" => BrowserFeature::Spdy,
      "speech-recognition" => BrowserFeature::SpeechRecognition,
      "speech-synthesis" => BrowserFeature::SpeechSynthesis,
      "spellcheck-attribute" => BrowserFeature::SpellcheckAttribute,
      "sql-storage" => BrowserFeature::SqlStorage,
      "srcset" => BrowserFeature::Srcset,
      "stream" => BrowserFeature::Stream,
      "streams" => BrowserFeature::Streams,
      "stricttransportsecurity" => BrowserFeature::Stricttransportsecurity,
      "style-scoped" => BrowserFeature::StyleScoped,
      "subresource-integrity" => BrowserFeature::SubresourceIntegrity,
      "svg" => BrowserFeature::Svg,
      "svg-css" => BrowserFeature::SvgCss,
      "svg-filters" => BrowserFeature::SvgFilters,
      "svg-fonts" => BrowserFeature::SvgFonts,
      "svg-fragment" => BrowserFeature::SvgFragment,
      "svg-html" => BrowserFeature::SvgHtml,
      "svg-html5" => BrowserFeature::SvgHtml5,
      "svg-img" => BrowserFeature::SvgImg,
      "svg-smil" => BrowserFeature::SvgSmil,
      "sxg" => BrowserFeature::Sxg,
      "tabindex-attr" => BrowserFeature::TabindexAttr,
      "template" => BrowserFeature::Template,
      "template-literals" => BrowserFeature::TemplateLiterals,
      "temporal" => BrowserFeature::Temporal,
      "text-decoration" => BrowserFeature::TextDecoration,
      "text-emphasis" => BrowserFeature::TextEmphasis,
      "text-overflow" => BrowserFeature::TextOverflow,
      "text-size-adjust" => BrowserFeature::TextSizeAdjust,
      "text-stroke" => BrowserFeature::TextStroke,
      "textcontent" => BrowserFeature::Textcontent,
      "textencoder" => BrowserFeature::Textencoder,
      "tls1-1" => BrowserFeature::Tls11,
      "tls1-2" => BrowserFeature::Tls12,
      "tls1-3" => BrowserFeature::Tls13,
      "touch" => BrowserFeature::Touch,
      "transforms2d" => BrowserFeature::Transforms2d,
      "transforms3d" => BrowserFeature::Transforms3d,
      "trusted-types" => BrowserFeature::TrustedTypes,
      "ttf" => BrowserFeature::Ttf,
      "typedarrays" => BrowserFeature::Typedarrays,
      "u2f" => BrowserFeature::U2f,
      "unhandledrejection" => BrowserFeature::Unhandledrejection,
      "upgradeinsecurerequests" => BrowserFeature::Upgradeinsecurerequests,
      "url" => BrowserFeature::Url,
      "url-scroll-to-text-fragment" => BrowserFeature::UrlScrollToTextFragment,
      "urlsearchparams" => BrowserFeature::Urlsearchparams,
      "use-strict" => BrowserFeature::UseStrict,
      "user-select-none" => BrowserFeature::UserSelectNone,
      "user-timing" => BrowserFeature::UserTiming,
      "variable-fonts" => BrowserFeature::VariableFonts,
      "vector-effect" => BrowserFeature::VectorEffect,
      "vibration" => BrowserFeature::Vibration,
      "video" => BrowserFeature::Video,
      "videotracks" => BrowserFeature::Videotracks,
      "view-transitions" => BrowserFeature::ViewTransitions,
      "viewport-unit-variants" => BrowserFeature::ViewportUnitVariants,
      "viewport-units" => BrowserFeature::ViewportUnits,
      "wai-aria" => BrowserFeature::WaiAria,
      "wake-lock" => BrowserFeature::WakeLock,
      "wasm" => BrowserFeature::Wasm,
      "wasm-bigint" => BrowserFeature::WasmBigint,
      "wasm-bulk-memory" => BrowserFeature::WasmBulkMemory,
      "wasm-multi-value" => BrowserFeature::WasmMultiValue,
      "wasm-mutable-globals" => BrowserFeature::WasmMutableGlobals,
      "wasm-nontrapping-fptoint" => BrowserFeature::WasmNontrappingFptoint,
      "wasm-reference-types" => BrowserFeature::WasmReferenceTypes,
      "wasm-signext" => BrowserFeature::WasmSignext,
      "wasm-simd" => BrowserFeature::WasmSimd,
      "wasm-threads" => BrowserFeature::WasmThreads,
      "wav" => BrowserFeature::Wav,
      "wbr-element" => BrowserFeature::WbrElement,
      "web-animation" => BrowserFeature::WebAnimation,
      "web-bluetooth" => BrowserFeature::WebBluetooth,
      "web-serial" => BrowserFeature::WebSerial,
      "web-share" => BrowserFeature::WebShare,
      "webauthn" => BrowserFeature::Webauthn,
      "webcodecs" => BrowserFeature::Webcodecs,
      "webgl" => BrowserFeature::Webgl,
      "webgl2" => BrowserFeature::Webgl2,
      "webgpu" => BrowserFeature::Webgpu,
      "webhid" => BrowserFeature::Webhid,
      "webkit-user-drag" => BrowserFeature::WebkitUserDrag,
      "webm" => BrowserFeature::Webm,
      "webnfc" => BrowserFeature::Webnfc,
      "webp" => BrowserFeature::Webp,
      "websockets" => BrowserFeature::Websockets,
      "webtransport" => BrowserFeature::Webtransport,
      "webusb" => BrowserFeature::Webusb,
      "webvr" => BrowserFeature::Webvr,
      "webvtt" => BrowserFeature::Webvtt,
      "webworkers" => BrowserFeature::Webworkers,
      "webxr" => BrowserFeature::Webxr,
      "will-change" => BrowserFeature::WillChange,
      "woff" => BrowserFeature::Woff,
      "woff2" => BrowserFeature::Woff2,
      "word-break" => BrowserFeature::WordBreak,
      "wordwrap" => BrowserFeature::Wordwrap,
      "x-doc-messaging" => BrowserFeature::XDocMessaging,
      "x-frame-options" => BrowserFeature::XFrameOptions,
      "xhr2" => BrowserFeature::Xhr2,
      "xhtml" => BrowserFeature::Xhtml,
      "xhtmlsmil" => BrowserFeature::Xhtmlsmil,
      "xml-serializer" => BrowserFeature::XmlSerializer,
      "zstd" => BrowserFeature::Zstd,
      key => BrowserFeature::Any(key.to_string()),
    }
  }
}

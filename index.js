async function onLoaded() {
  const params = new URLSearchParams(document.location.search);
  const token = params.get("token");
  if (token != null) {
    setAccessToken(token);
    window.location.href = "/";
  }

  const accessToken = getAccessToken();
  if (accessToken == null) {
    document.location.href = "/spotify/auth";
    return;
  }

  try {
    const np = await getNp(accessToken);
    if (np == null) {
      const elm = document.createElement("p");
      elm.innerText = "今なにも再生してないっぽい!";
      document.getElementById("root").appendChild(elm);
      return;
    }
    const text = getNpText(np);
    redirectCompose(text, np.trackUrl);
  } catch (e) {
    console.error(e);
  }
}

function getAccessToken() {
  return localStorage.getItem("NPBTN_TOKEN");
}

function setAccessToken(text) {
  localStorage.setItem("NPBTN_TOKEN", text);
}

async function getNp(token) {
  const res = await fetch("/np?token=" + encodeURIComponent(token));
  return res.json();
}

function redirectCompose(text, url) {
  const redirectUrl = `https://twitter.com/intent/tweet?text=${text}&url=${url}`;
  document.location.href = redirectUrl;
}

function getNpText(np) {
  console.log(np);
  const text = `NowPlaying ${np.artistNames.join(", ")} - ${np.trackName}`;
  return text;
}

onLoaded();

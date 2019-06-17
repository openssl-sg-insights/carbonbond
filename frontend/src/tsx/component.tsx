import * as React from "react";

import "../css/layout.css";

import { Login } from "./types";

function Component(): JSX.Element {
	const context = React.useContext(Login);
	return (
		<div styleName="app">
			<div styleName="header">yo</div>
			<div styleName="menubar">yo</div>
			<div styleName="sidebar">yo</div>
			<div styleName="mainContent">
				<h1>金剛、石墨，參見！</h1>
				<h1>{context.login ? context.user_id : "未登入"}</h1>
				{
					(() => {
						if (context.login) {
							return <button className="pure-button"
								onClick={context.unsetLogin}>登出</button>;
						} else {
							return <button className="pure-button"
								onClick={() => context.setLogin("測試帳號")}>登入</button>;
						}
					})()
				}
				<button className="pure-button">註冊</button>
			</div>
		</div>
	);
}

export { Component };
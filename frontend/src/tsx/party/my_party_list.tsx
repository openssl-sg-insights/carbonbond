import * as React from 'react';
import { Navigate, Link, useNavigate } from 'react-router-dom';
import { useTitle } from 'react-use';

import { UserState } from '../global_state/user';
import { LocationCacheState } from '../global_state/board_cache';
import style from '../../css/party/my_party_list.module.css';
import { API_FETCHER, unwrap_or, unwrap } from '../../ts/api/api';
import { Party } from '../../ts/api/api_trait';

import { EXILED_PARTY_NAME } from './index';
import { toastErr } from '../utils';

async function fetchPartyList(): Promise<Party[]> {
	let party_list = unwrap_or(await API_FETCHER.userQuery.queryMyPartyList(), []);
	return party_list;
}

// TODO: 再發一個請求取得看板資訊
export function MyPartyList(): JSX.Element {
	let [fetching, setFetching] = React.useState(true);
	let [party_list, setPartyList] = React.useState<Party[]>([]);
	let { user_state } = UserState.useContainer();
	const { setCurrentLocation } = LocationCacheState.useContainer();

	React.useEffect(() => {
		fetchPartyList().then(tree => {
			setPartyList(tree);
			setFetching(false);
		}).catch(err => toastErr(err));
	}, []);

	React.useEffect(() => {
		setCurrentLocation({name: '我的政黨', is_board: false});
	}, [setCurrentLocation]);
	useTitle('我的政黨');

	if (!user_state.login && !user_state.fetching) {
		return <Navigate to="/app" />;
	} if (fetching) {
		return <div></div>;
	} else {
		return <div className={style.listBody}>
			{
				party_list.map(party => {
					return <div key={party.id} className={style.boardPartyBlock}>
						{
							(() => {
								if (party.board_id == null) {
									return <div className={style.boardName}>{EXILED_PARTY_NAME}</div>;
								} else {
									// XXX: 補看板名
									let href = `/app/board/${party.board_name}`;
									return <Link to={href} className={style.boardName}>
										<div className={style.boardName}>{party.board_name}</div>
									</Link>;
								}
							})()
						}
						<Link
							to={`/app/party/${party.party_name}`}
							key={party.id}
							className={style.partyColumn}
						>
							<div className={style.ruling}>{party.ruling ? '執政 ' : ''}</div>
							<div className={style.partyLabel}>{party.party_name}</div>
							<div className={style.partyLabel}>☘ {party.energy}</div>
							{/* <div className={style.partyLabel}>👑{party.chairmanId}</div> */}
							{/* <div className={style.partyLabel}>📊 10%</div> */}
						</Link>
					</div>;
				})
			}
			<CreatePartyBlock />
		</div>;
	}
}

function CreatePartyBlock(): JSX.Element {
	let [expand, setExpand] = React.useState(false);
	let [party_name, setPartyName] = React.useState('');
	let [board_name, setBoardName] = React.useState('');
	let navigate = useNavigate();
	return <>
		<div onClick={() => setExpand(!expand)} className={style.createParty}> 👥 創建政黨 </div>
		<div style={{ display: expand ? 'block' : 'none', textAlign: 'right' }}>
			<input type="text"
				value={party_name}
				placeholder="政黨名稱"
				className={style.createPartyInput}
				onChange={evt => {
					setPartyName(evt.target.value);
					// TODO: 向後端詢問
				}}
			/>
			<input type="text"
				value={board_name}
				placeholder="依附於看板（預設為流亡政黨）"
				className={style.createPartyInput}
				onChange={evt => {
					setBoardName(evt.target.value);
					// TODO: 向後端詢問
				}}
			/>
			<br />
			<button onClick={() => {
				API_FETCHER.partyQuery.createParty(
					party_name,
					board_name.length == 0 ? null : board_name,
				).then(res => {
					unwrap(res);
				}).then(() => {
					navigate(`/app/party/${party_name}`);
				}).catch(err => {
					toastErr(err);
				});
			}}>確認</button>
		</div>
	</>;
}
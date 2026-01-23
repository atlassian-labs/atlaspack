function InfoCard({title, children, icon}) {
  return (
    <div className="info-card">
      {icon && <div className="info-card-icon">{icon}</div>}
      <div className="info-card-content">
        {title && <h4>{title}</h4>}
        <div className="info-card-body">{children}</div>
      </div>
    </div>
  );
}

module.exports = InfoCard;
